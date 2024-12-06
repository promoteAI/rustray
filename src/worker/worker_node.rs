//! 工作节点模块
//! 
//! 本模块实现了分布式系统中的工作节点，支持：
//! - 任务执行和管理
//! - 资源监控和管理
//! - 心跳和健康检查
//! - 故障恢复
//! - 性能优化

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::common::{
    NodeInfo, NodeType, TaskSpec, TaskResult, TaskRequiredResources,
    object_store::ObjectStore,
};
use crate::metrics::MetricsCollector;

/// 工作节点状态
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerState {
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 暂停（维护模式）
    Paused,
    /// 关闭中
    ShuttingDown,
    /// 已关闭
    Shutdown,
    /// 错误状态
    Error(String),
}

/// 工作节点配置
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// 节点地址
    pub address: String,
    /// 节点端口
    pub port: u16,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 心跳间隔
    pub heartbeat_interval: Duration,
    /// 资源监控间隔
    pub resource_monitor_interval: Duration,
    /// 重连策略配置
    pub reconnect_config: ReconnectConfig,
}

/// 重连策略配置
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// 初始重试延迟
    pub initial_delay: Duration,
    /// 最大重试延迟
    pub max_delay: Duration,
    /// 最大重试次数
    pub max_attempts: usize,
    /// 退避因子
    pub backoff_factor: f64,
}

/// 工作节点
pub struct WorkerNode {
    /// 节点信息
    pub node_info: NodeInfo,
    /// 节点配置
    config: WorkerConfig,
    /// 节点状态
    state: Arc<Mutex<WorkerState>>,
    /// 当前运行的任务
    running_tasks: Arc<Mutex<HashMap<String, TaskInfo>>>,
    /// 可用资源
    available_resources: Arc<Mutex<TaskRequiredResources>>,
    /// 对象存储
    object_store: Arc<ObjectStore>,
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 状态更新通道
    status_tx: mpsc::Sender<WorkerStatus>,
    /// 后台任务句柄
    background_tasks: Vec<JoinHandle<()>>,
}

/// 任务信息
#[derive(Debug)]
struct TaskInfo {
    /// 任务规范
    spec: TaskSpec,
    /// 开始时间
    start_time: Instant,
    /// 资源使用情况
    resources: TaskRequiredResources,
    /// 任务句柄
    handle: JoinHandle<()>,
}

/// 工作节点状态
#[derive(Debug, Clone)]
pub struct WorkerStatus {
    /// 节点ID
    pub node_id: String,
    /// 节点状态
    pub state: WorkerState,
    /// 运行中的任务数
    pub running_tasks: usize,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用量
    pub memory_usage: usize,
    /// 最后心跳时间
    pub last_heartbeat: Instant,
}

impl WorkerNode {
    /// 创建新的工作节点
    pub fn new(
        config: WorkerConfig,
        object_store: Arc<ObjectStore>,
        metrics: Arc<MetricsCollector>,
        status_tx: mpsc::Sender<WorkerStatus>,
    ) -> Self {
        let node_info = NodeInfo {
            node_id: uuid::Uuid::new_v4(),
            node_type: NodeType::Worker,
            address: config.address.clone(),
            port: config.port,
        };

        Self {
            node_info,
            config,
            state: Arc::new(Mutex::new(WorkerState::Initializing)),
            running_tasks: Arc::new(Mutex::new(HashMap::new())),
            available_resources: Arc::new(Mutex::new(TaskRequiredResources::default())),
            object_store,
            metrics,
            status_tx,
            background_tasks: Vec::new(),
        }
    }

    /// 启动工作节点
    pub async fn start(&mut self) -> Result<(), String> {
        info!("Starting worker node: {}", self.node_info.node_id);

        // 初始化资源监控
        self.init_resource_monitor()?;

        // 启动心跳服务
        self.start_heartbeat_service()?;

        // 更新节点状态
        *self.state.lock().map_err(|e| e.to_string())? = WorkerState::Running;
        
        self.update_status().await?;
        info!("Worker node started successfully");
        Ok(())
    }

    /// 停止工作节点
    pub async fn stop(&mut self) -> Result<(), String> {
        info!("Stopping worker node: {}", self.node_info.node_id);

        // 更新状态为关闭中
        *self.state.lock().map_err(|e| e.to_string())? = WorkerState::ShuttingDown;
        
        // 等待所有任务完成
        self.wait_for_tasks().await?;

        // 停止所有后台任务
        for task in self.background_tasks.drain(..) {
            task.abort();
        }

        // 更新状态为已关闭
        *self.state.lock().map_err(|e| e.to_string())? = WorkerState::Shutdown;
        
        self.update_status().await?;
        info!("Worker node stopped successfully");
        Ok(())
    }

    /// 执行任务
    pub async fn execute_task(&self, task: TaskSpec) -> Result<TaskResult, String> {
        let task_id = task.task_id.clone();
        info!("Executing task: {}", task_id);

        // 检查资源是否满足要求
        self.check_resources(&task.required_resources)?;

        // 分配资源
        self.allocate_resources(&task.required_resources)?;

        let start_time = Instant::now();
        let task_resources = task.required_resources.clone();

        // 创建任务执行器
        let executor = TaskExecutor::new(
            task.clone(),
            self.object_store.clone(),
            self.metrics.clone(),
        );

        // 启动任务
        let handle = tokio::spawn(async move {
            executor.execute().await;
        });

        // 记录任务信息
        let task_info = TaskInfo {
            spec: task,
            start_time,
            resources: task_resources,
            handle,
        };

        self.running_tasks.lock().map_err(|e| e.to_string())?
            .insert(task_id.clone(), task_info);

        // 更新指标
        self.metrics.increment_counter("worker.tasks.started", 1)
            .map_err(|e| e.to_string())?;

        self.update_status().await?;
        Ok(TaskResult::Running)
    }

    /// 获取节点状态
    pub fn get_state(&self) -> Result<WorkerState, String> {
        Ok(self.state.lock().map_err(|e| e.to_string())?.clone())
    }

    /// 更新可用资源
    pub fn update_resources(&self, resources: TaskRequiredResources) -> Result<(), String> {
        *self.available_resources.lock().map_err(|e| e.to_string())? = resources;
        Ok(())
    }

    // 私有辅助方法

    /// 初始化资源监控
    fn init_resource_monitor(&mut self) -> Result<(), String> {
        let state = self.state.clone();
        let metrics = self.metrics.clone();
        let interval = self.config.resource_monitor_interval;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                
                if *state.lock().unwrap() != WorkerState::Running {
                    break;
                }

                // 收集系统资源使用情况
                if let Ok(resources) = sys_info::loadavg() {
                    metrics.set_gauge("worker.cpu.load1", resources.one)
                        .unwrap_or_else(|e| warn!("Failed to update CPU metrics: {}", e));
                }

                if let Ok(memory) = sys_info::mem_info() {
                    metrics.set_gauge("worker.memory.total", memory.total as f64)
                        .unwrap_or_else(|e| warn!("Failed to update memory metrics: {}", e));
                    metrics.set_gauge("worker.memory.free", memory.free as f64)
                        .unwrap_or_else(|e| warn!("Failed to update memory metrics: {}", e));
                }
            }
        });

        self.background_tasks.push(handle);
        Ok(())
    }

    /// 启动心跳服务
    fn start_heartbeat_service(&mut self) -> Result<(), String> {
        let state = self.state.clone();
        let metrics = self.metrics.clone();
        let node_id = self.node_info.node_id.to_string();
        let interval = self.config.heartbeat_interval;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                
                if *state.lock().unwrap() != WorkerState::Running {
                    break;
                }

                // 发送心跳
                metrics.increment_counter("worker.heartbeat.sent", 1)
                    .unwrap_or_else(|e| warn!("Failed to update heartbeat metrics: {}", e));

                debug!("Sent heartbeat for node: {}", node_id);
            }
        });

        self.background_tasks.push(handle);
        Ok(())
    }

    /// 检查资源是否满足要求
    fn check_resources(&self, required: &TaskRequiredResources) -> Result<(), String> {
        let available = self.available_resources.lock().map_err(|e| e.to_string())?;

        if let Some(req_cpu) = required.cpu {
            if let Some(avail_cpu) = available.cpu {
                if avail_cpu < req_cpu {
                    return Err("Insufficient CPU resources".to_string());
                }
            }
        }

        if let Some(req_mem) = required.memory {
            if let Some(avail_mem) = available.memory {
                if avail_mem < req_mem {
                    return Err("Insufficient memory resources".to_string());
                }
            }
        }

        if let Some(req_gpu) = required.gpu {
            if let Some(avail_gpu) = available.gpu {
                if avail_gpu < req_gpu {
                    return Err("Insufficient GPU resources".to_string());
                }
            }
        }

        Ok(())
    }

    /// 分配资源
    fn allocate_resources(&self, resources: &TaskRequiredResources) -> Result<(), String> {
        let mut available = self.available_resources.lock().map_err(|e| e.to_string())?;

        if let Some(req_cpu) = resources.cpu {
            if let Some(ref mut avail_cpu) = available.cpu {
                *avail_cpu -= req_cpu;
            }
        }

        if let Some(req_mem) = resources.memory {
            if let Some(ref mut avail_mem) = available.memory {
                *avail_mem -= req_mem;
            }
        }

        if let Some(req_gpu) = resources.gpu {
            if let Some(ref mut avail_gpu) = available.gpu {
                *avail_gpu -= req_gpu;
            }
        }

        Ok(())
    }

    /// 等待所有任务完成
    async fn wait_for_tasks(&self) -> Result<(), String> {
        let tasks = self.running_tasks.lock().map_err(|e| e.to_string())?;
        for (task_id, task_info) in tasks.iter() {
            info!("Waiting for task to complete: {}", task_id);
            if let Err(e) = task_info.handle.await {
                error!("Task {} failed: {}", task_id, e);
            }
        }
        Ok(())
    }

    /// 更新节点状态
    async fn update_status(&self) -> Result<(), String> {
        let status = WorkerStatus {
            node_id: self.node_info.node_id.to_string(),
            state: self.state.lock().map_err(|e| e.to_string())?.clone(),
            running_tasks: self.running_tasks.lock().map_err(|e| e.to_string())?.len(),
            cpu_usage: sys_info::loadavg().map(|l| l.one).unwrap_or(0.0),
            memory_usage: sys_info::mem_info().map(|m| m.total - m.free).unwrap_or(0),
            last_heartbeat: Instant::now(),
        };

        self.status_tx.send(status).await
            .map_err(|e| e.to_string())
    }
}

/// 任务执行器
struct TaskExecutor {
    task: TaskSpec,
    object_store: Arc<ObjectStore>,
    metrics: Arc<MetricsCollector>,
}

impl TaskExecutor {
    fn new(
        task: TaskSpec,
        object_store: Arc<ObjectStore>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            task,
            object_store,
            metrics,
        }
    }

    async fn execute(&self) {
        let start_time = Instant::now();

        // 执行任务逻辑
        // ...

        // 更新指标
        let duration = start_time.elapsed();
        self.metrics.record_histogram("worker.task.duration", duration.as_secs_f64())
            .unwrap_or_else(|e| error!("Failed to update task metrics: {}", e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> WorkerConfig {
        WorkerConfig {
            address: "localhost".to_string(),
            port: 8000,
            max_concurrent_tasks: 4,
            heartbeat_interval: Duration::from_secs(1),
            resource_monitor_interval: Duration::from_secs(1),
            reconnect_config: ReconnectConfig {
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(60),
                max_attempts: 3,
                backoff_factor: 2.0,
            },
        }
    }

    #[tokio::test]
    async fn test_worker_creation() {
        let (tx, _) = mpsc::channel(100);
        let object_store = Arc::new(ObjectStore::new("test".to_string()));
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        
        let worker = WorkerNode::new(
            create_test_config(),
            object_store,
            metrics,
            tx,
        );

        assert_eq!(
            *worker.state.lock().unwrap(),
            WorkerState::Initializing
        );
    }

    #[tokio::test]
    async fn test_worker_lifecycle() {
        let (tx, _) = mpsc::channel(100);
        let object_store = Arc::new(ObjectStore::new("test".to_string()));
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        
        let mut worker = WorkerNode::new(
            create_test_config(),
            object_store,
            metrics,
            tx,
        );

        // 测试启动
        let result = worker.start().await;
        assert!(result.is_ok());
        assert_eq!(
            *worker.state.lock().unwrap(),
            WorkerState::Running
        );

        // 测试停止
        let result = worker.stop().await;
        assert!(result.is_ok());
        assert_eq!(
            *worker.state.lock().unwrap(),
            WorkerState::Shutdown
        );
    }

    #[tokio::test]
    async fn test_resource_management() {
        let (tx, _) = mpsc::channel(100);
        let object_store = Arc::new(ObjectStore::new("test".to_string()));
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        
        let worker = WorkerNode::new(
            create_test_config(),
            object_store,
            metrics,
            tx,
        );

        // 更新可用资源
        let resources = TaskRequiredResources {
            cpu: Some(4.0),
            memory: Some(8192),
            gpu: Some(1),
        };
        let result = worker.update_resources(resources);
        assert!(result.is_ok());

        // 检查资源分配
        let required = TaskRequiredResources {
            cpu: Some(2.0),
            memory: Some(4096),
            gpu: None,
        };
        let result = worker.check_resources(&required);
        assert!(result.is_ok());
    }
} 