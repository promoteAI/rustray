//! 头节点模块
//! 
//! 本模块实现了分布式系统的头节点，负责：
//! - 集群管理和协调
//! - 任务调度和分发
//! - 资源管理和分配
//! - 故障检测和恢复
//! - 性能监控和优化

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::common::{
    NodeInfo, NodeType, TaskSpec, TaskResult, TaskRequiredResources,
    object_store::ObjectStore,
};
use crate::metrics::MetricsCollector;
use crate::scheduler::{TaskGraph, LoadBalancer, LoadBalanceStrategy};

/// 头节点配置
#[derive(Debug, Clone)]
pub struct HeadNodeConfig {
    /// 节点地址
    pub address: String,
    /// 节点端口
    pub port: u16,
    /// 心跳超时时间
    pub heartbeat_timeout: Duration,
    /// 资源监控间隔
    pub resource_monitor_interval: Duration,
    /// 调度策略
    pub scheduling_strategy: LoadBalanceStrategy,
}

/// 头节点状态
#[derive(Debug, Clone, PartialEq)]
pub enum HeadNodeState {
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 降级模式
    Degraded(String),
    /// 关闭中
    ShuttingDown,
    /// 已关闭
    Shutdown,
    /// 错误状态
    Error(String),
}

/// 集群状态
#[derive(Debug, Clone)]
pub struct ClusterStatus {
    /// 总节点数
    pub total_nodes: usize,
    /// 活跃节点数
    pub active_nodes: usize,
    /// 总任务数
    pub total_tasks: usize,
    /// 运行中的任务数
    pub running_tasks: usize,
    /// 等待中的任务数
    pub pending_tasks: usize,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用量
    pub memory_usage: usize,
}

/// 头节点
pub struct HeadNode {
    /// 节点信息
    pub node_info: NodeInfo,
    /// 节点配置
    config: HeadNodeConfig,
    /// 节点状态
    state: Arc<Mutex<HeadNodeState>>,
    /// 工作节点映射
    workers: Arc<Mutex<HashMap<String, WorkerInfo>>>,
    /// 任务图
    task_graph: Arc<TaskGraph>,
    /// 负载均衡器
    load_balancer: Arc<LoadBalancer>,
    /// 对象存储
    object_store: Arc<ObjectStore>,
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 状态更新通道
    status_tx: mpsc::Sender<ClusterStatus>,
    /// 后台任务句柄
    background_tasks: Vec<tokio::task::JoinHandle<()>>,
}

/// 工作节点信息
#[derive(Debug, Clone)]
struct WorkerInfo {
    /// 节点信息
    node_info: NodeInfo,
    /// 最后心跳时间
    last_heartbeat: Instant,
    /// 资源使用情况
    resources: TaskRequiredResources,
    /// 运行中的任务数
    running_tasks: usize,
}

impl HeadNode {
    /// 创建新的头节点
    pub fn new(
        config: HeadNodeConfig,
        object_store: Arc<ObjectStore>,
        metrics: Arc<MetricsCollector>,
        status_tx: mpsc::Sender<ClusterStatus>,
    ) -> Self {
        let node_info = NodeInfo {
            node_id: uuid::Uuid::new_v4(),
            node_type: NodeType::Head,
            address: config.address.clone(),
            port: config.port,
        };

        let (task_graph_tx, _) = mpsc::channel(100);
        let (load_balancer_tx, _) = mpsc::channel(100);

        let task_graph = Arc::new(TaskGraph::new(
            metrics.clone(),
            task_graph_tx,
        ));

        let load_balancer = Arc::new(LoadBalancer::new(
            config.scheduling_strategy.clone(),
            metrics.clone(),
            load_balancer_tx,
        ));

        Self {
            node_info,
            config,
            state: Arc::new(Mutex::new(HeadNodeState::Initializing)),
            workers: Arc::new(Mutex::new(HashMap::new())),
            task_graph,
            load_balancer,
            object_store,
            metrics,
            status_tx,
            background_tasks: Vec::new(),
        }
    }

    /// 启动头节点
    pub async fn start(&mut self) -> Result<(), String> {
        info!("Starting head node: {}", self.node_info.node_id);

        // 启动心跳检查服务
        self.start_heartbeat_checker()?;

        // 启动资源监控服务
        self.start_resource_monitor()?;

        // 启动调度服务
        self.start_scheduler()?;

        // 更新节点状态
        *self.state.lock().map_err(|e| e.to_string())? = HeadNodeState::Running;
        
        self.update_status().await?;
        info!("Head node started successfully");
        Ok(())
    }

    /// 停止头节点
    pub async fn stop(&mut self) -> Result<(), String> {
        info!("Stopping head node: {}", self.node_info.node_id);

        // 更新状态为关闭中
        *self.state.lock().map_err(|e| e.to_string())? = HeadNodeState::ShuttingDown;

        // 停止所有后台任务
        for task in self.background_tasks.drain(..) {
            task.abort();
        }

        // 更新状态为已关闭
        *self.state.lock().map_err(|e| e.to_string())? = HeadNodeState::Shutdown;
        
        self.update_status().await?;
        info!("Head node stopped successfully");
        Ok(())
    }

    /// 注册工作节点
    pub async fn register_worker(&self, worker: NodeInfo) -> Result<(), String> {
        info!("Registering worker node: {}", worker.node_id);

        let worker_info = WorkerInfo {
            node_info: worker.clone(),
            last_heartbeat: Instant::now(),
            resources: TaskRequiredResources::default(),
            running_tasks: 0,
        };

        // 添加到工作节点列表
        self.workers.lock().map_err(|e| e.to_string())?
            .insert(worker.node_id.to_string(), worker_info);

        // 注册到负载均衡器
        self.load_balancer.register_node(worker)?;

        // 更新指标
        self.metrics.increment_counter("head.workers.registered", 1)
            .map_err(|e| e.to_string())?;

        self.update_status().await?;
        info!("Worker node registered successfully");
        Ok(())
    }

    /// 提交任务
    pub async fn submit_task(&self, task: TaskSpec) -> Result<(), String> {
        info!("Submitting task: {}", task.task_id);

        // 添加到任务图
        self.task_graph.add_task(task.clone(), vec![])?;

        // 选择工作节点
        if let Some(worker_id) = self.load_balancer.select_node(&task) {
            let workers = self.workers.lock().map_err(|e| e.to_string())?;
            if let Some(worker) = workers.get(&worker_id) {
                // 发送任务到工作节点
                // TODO: 实现任务发送逻辑
                
                // 更新指标
                self.metrics.increment_counter("head.tasks.submitted", 1)
                    .map_err(|e| e.to_string())?;
            }
        }

        self.update_status().await?;
        Ok(())
    }

    /// 更新工作节点心跳
    pub async fn update_worker_heartbeat(
        &self,
        worker_id: &str,
        resources: TaskRequiredResources,
    ) -> Result<(), String> {
        let mut workers = self.workers.lock().map_err(|e| e.to_string())?;
        
        if let Some(worker) = workers.get_mut(worker_id) {
            worker.last_heartbeat = Instant::now();
            worker.resources = resources;

            // 更新负载均衡器中的节点状态
            self.load_balancer.update_node_state(
                worker_id,
                crate::scheduler::NodeHealth::Healthy,
                Some(worker.running_tasks),
                None,
                Some(resources),
            )?;

            // 更新指标
            self.metrics.increment_counter("head.heartbeats.received", 1)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    // 私有辅助方法

    /// 启动心跳检查服务
    fn start_heartbeat_checker(&mut self) -> Result<(), String> {
        let state = self.state.clone();
        let workers = self.workers.clone();
        let load_balancer = self.load_balancer.clone();
        let metrics = self.metrics.clone();
        let timeout = self.config.heartbeat_timeout;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(timeout);
            loop {
                interval.tick().await;
                
                if *state.lock().unwrap() != HeadNodeState::Running {
                    break;
                }

                let mut workers = workers.lock().unwrap();
                let now = Instant::now();

                // 检查心跳超时的节点
                workers.retain(|worker_id, info| {
                    let is_alive = now.duration_since(info.last_heartbeat) < timeout;
                    if !is_alive {
                        warn!("Worker node {} heartbeat timeout", worker_id);
                        
                        // 更新负载均衡器
                        if let Err(e) = load_balancer.update_node_state(
                            worker_id,
                            crate::scheduler::NodeHealth::Unhealthy("Heartbeat timeout".to_string()),
                            None,
                            None,
                            None,
                        ) {
                            error!("Failed to update node state: {}", e);
                        }

                        metrics.increment_counter("head.workers.timeout", 1)
                            .unwrap_or_else(|e| error!("Failed to update metrics: {}", e));
                    }
                    is_alive
                });
            }
        });

        self.background_tasks.push(handle);
        Ok(())
    }

    /// 启动资源监控服务
    fn start_resource_monitor(&mut self) -> Result<(), String> {
        let state = self.state.clone();
        let workers = self.workers.clone();
        let metrics = self.metrics.clone();
        let interval = self.config.resource_monitor_interval;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                
                if *state.lock().unwrap() != HeadNodeState::Running {
                    break;
                }

                let workers = workers.lock().unwrap();
                
                // 计算集群资源使用情况
                let mut total_cpu = 0.0;
                let mut total_memory = 0;
                let mut active_workers = 0;

                for worker in workers.values() {
                    if let Some(cpu) = worker.resources.cpu {
                        total_cpu += cpu;
                    }
                    if let Some(memory) = worker.resources.memory {
                        total_memory += memory;
                    }
                    active_workers += 1;
                }

                // 更新指标
                if active_workers > 0 {
                    let avg_cpu = total_cpu / active_workers as f64;
                    let avg_memory = total_memory / active_workers;

                    metrics.set_gauge("head.cluster.cpu_usage", avg_cpu)
                        .unwrap_or_else(|e| error!("Failed to update CPU metrics: {}", e));
                    metrics.set_gauge("head.cluster.memory_usage", avg_memory as f64)
                        .unwrap_or_else(|e| error!("Failed to update memory metrics: {}", e));
                }
            }
        });

        self.background_tasks.push(handle);
        Ok(())
    }

    /// 启动调度服务
    fn start_scheduler(&mut self) -> Result<(), String> {
        let state = self.state.clone();
        let task_graph = self.task_graph.clone();
        let load_balancer = self.load_balancer.clone();
        let metrics = self.metrics.clone();

        let handle = tokio::spawn(async move {
            loop {
                if *state.lock().unwrap() != HeadNodeState::Running {
                    break;
                }

                // 获取下一个可执行的任务
                if let Some(task) = task_graph.get_next_task() {
                    // 选择工作节点
                    if let Some(worker_id) = load_balancer.select_node(&task) {
                        // TODO: 实现任务分发逻辑
                        
                        metrics.increment_counter("head.tasks.scheduled", 1)
                            .unwrap_or_else(|e| error!("Failed to update metrics: {}", e));
                    }
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        self.background_tasks.push(handle);
        Ok(())
    }

    /// 更新集群状态
    async fn update_status(&self) -> Result<(), String> {
        let workers = self.workers.lock().map_err(|e| e.to_string())?;
        
        let mut total_cpu_usage = 0.0;
        let mut total_memory_usage = 0;
        let mut active_workers = 0;
        let mut running_tasks = 0;

        for worker in workers.values() {
            if worker.last_heartbeat.elapsed() < self.config.heartbeat_timeout {
                active_workers += 1;
                running_tasks += worker.running_tasks;

                if let Some(cpu) = worker.resources.cpu {
                    total_cpu_usage += cpu;
                }
                if let Some(memory) = worker.resources.memory {
                    total_memory_usage += memory;
                }
            }
        }

        let status = ClusterStatus {
            total_nodes: workers.len(),
            active_nodes: active_workers,
            total_tasks: self.task_graph.get_total_tasks()?,
            running_tasks,
            pending_tasks: self.task_graph.get_pending_tasks()?,
            cpu_usage: if active_workers > 0 {
                total_cpu_usage / active_workers as f64
            } else {
                0.0
            },
            memory_usage: if active_workers > 0 {
                total_memory_usage / active_workers
            } else {
                0
            },
        };

        self.status_tx.send(status).await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> HeadNodeConfig {
        HeadNodeConfig {
            address: "localhost".to_string(),
            port: 8000,
            heartbeat_timeout: Duration::from_secs(5),
            resource_monitor_interval: Duration::from_secs(1),
            scheduling_strategy: LoadBalanceStrategy::RoundRobin,
        }
    }

    #[tokio::test]
    async fn test_head_node_creation() {
        let (tx, _) = mpsc::channel(100);
        let object_store = Arc::new(ObjectStore::new("test".to_string()));
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        
        let head = HeadNode::new(
            create_test_config(),
            object_store,
            metrics,
            tx,
        );

        assert_eq!(
            *head.state.lock().unwrap(),
            HeadNodeState::Initializing
        );
    }

    #[tokio::test]
    async fn test_worker_registration() {
        let (tx, _) = mpsc::channel(100);
        let object_store = Arc::new(ObjectStore::new("test".to_string()));
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        
        let head = HeadNode::new(
            create_test_config(),
            object_store,
            metrics,
            tx,
        );

        let worker = NodeInfo {
            node_id: uuid::Uuid::new_v4(),
            node_type: NodeType::Worker,
            address: "localhost".to_string(),
            port: 8001,
        };

        let result = head.register_worker(worker.clone()).await;
        assert!(result.is_ok());
        
        let workers = head.workers.lock().unwrap();
        assert_eq!(workers.len(), 1);
        assert!(workers.contains_key(&worker.node_id.to_string()));
    }

    #[tokio::test]
    async fn test_task_submission() {
        let (tx, _) = mpsc::channel(100);
        let object_store = Arc::new(ObjectStore::new("test".to_string()));
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        
        let head = HeadNode::new(
            create_test_config(),
            object_store,
            metrics,
            tx,
        );

        let task = TaskSpec {
            task_id: uuid::Uuid::new_v4().to_string(),
            function_name: "test_function".to_string(),
            args: vec![],
            kwargs: HashMap::new(),
            required_resources: TaskRequiredResources::default(),
            ..Default::default()
        };

        let result = head.submit_task(task).await;
        assert!(result.is_ok());
    }
} 