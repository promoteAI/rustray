pub mod load_balancer;
pub mod task_graph;
pub mod data_aware_scheduler;

use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use tokio::sync::{RwLock, mpsc};
use anyhow::{Result, anyhow};
use uuid::Uuid;
use std::time::{Duration, Instant};

use crate::common::{TaskSpec, TaskResult};
use crate::worker::WorkerNode;
use self::task_graph::{TaskGraph, TaskState};
use self::data_aware_scheduler::{DataAwareScheduler, WorkerResources};

#[derive(Debug, Clone, PartialEq)]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_retries: usize,
    pub retry_delay: Duration,
    pub preemption_threshold: TaskPriority,
    pub resource_overcommit_factor: f64,
    pub min_worker_ready_time: Duration,
    pub health_check_interval: Duration,
    pub load_balance_interval: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            preemption_threshold: TaskPriority::High,
            resource_overcommit_factor: 1.2,
            min_worker_ready_time: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            load_balance_interval: Duration::from_secs(60),
        }
    }
}

/// 全局调度器
pub struct GlobalScheduler {
    task_graph: Arc<TaskGraph>,
    data_scheduler: Arc<DataAwareScheduler>,
    workers: Arc<RwLock<Vec<WorkerNode>>>,
    config: SchedulerConfig,
    
    // 任务重试状态
    retry_counts: Arc<RwLock<HashMap<String, usize>>>,
    
    // 资源预留
    reservations: Arc<RwLock<HashMap<String, Vec<ResourceReservation>>>>,
    
    // 工作节点健康状态
    worker_health: Arc<RwLock<HashMap<String, WorkerHealth>>>,
    
    // 调度统计
    stats: Arc<RwLock<SchedulerStats>>,
    
    // 任务队列（按优先级）
    task_queues: Arc<RwLock<HashMap<TaskPriority, Vec<TaskSpec>>>>,
    
    // 工作流管理
    workflows: Arc<RwLock<HashMap<String, Workflow>>>,
    
    // 任务缓存
    task_cache: Arc<RwLock<HashMap<String, TaskCache>>>,
    
    // 资源预测器
    resource_predictor: Arc<ResourcePredictor>,
    
    // 可插拔调度策略
    scheduling_strategy: Arc<Box<dyn SchedulingStrategy>>,
}

#[derive(Debug, Clone)]
struct ResourceReservation {
    task_id: String,
    resources: WorkerResources,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct WorkerHealth {
    last_heartbeat: Instant,
    failed_heartbeats: usize,
    total_tasks: usize,
    failed_tasks: usize,
    avg_task_duration: Duration,
}

#[derive(Debug, Default)]
struct SchedulerStats {
    tasks_scheduled: usize,
    tasks_completed: usize,
    tasks_failed: usize,
    tasks_preempted: usize,
    total_scheduling_time: Duration,
    resource_utilization: HashMap<String, f64>,
}

// 工作流相关结构
#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub tasks: Vec<WorkflowTask>,
    pub dependencies: HashMap<String, Vec<String>>,  // task_id -> [dependent_task_ids]
    pub state: WorkflowState,
    pub created_at: Instant,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct WorkflowTask {
    pub id: String,
    pub task_spec: TaskSpec,
    pub state: TaskState,
    pub retry_strategy: RetryStrategy,
    pub cache_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowState {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

// 重试策略
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    pub max_attempts: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
    pub retry_on_errors: Vec<String>,
}

// 缓存相关结构
#[derive(Debug, Clone)]
pub struct TaskCache {
    pub key: String,
    pub result: TaskResult,
    pub created_at: Instant,
    pub ttl: Duration,
    pub size: usize,
}

// 资源预测相关结构
#[derive(Debug, Clone)]
pub struct ResourcePrediction {
    pub cpu_usage: Vec<(Instant, f64)>,
    pub memory_usage: Vec<(Instant, usize)>,
    pub network_usage: Vec<(Instant, (u64, u64))>,  // (rx, tx)
    pub confidence: f64,
}

// 调度策略接口
#[async_trait]
pub trait SchedulingStrategy: Send + Sync {
    async fn select_worker(&self, task: &TaskSpec, workers: &[WorkerNode]) -> Result<String>;
    async fn should_preempt(&self, new_task: &TaskSpec, running_task: &TaskSpec) -> bool;
    async fn calculate_priority(&self, task: &TaskSpec) -> TaskPriority;
}

impl GlobalScheduler {
    pub fn new() -> Self {
        Self::with_config(SchedulerConfig::default())
    }

    pub fn with_config(config: SchedulerConfig) -> Self {
        let scheduler = Self {
            task_graph: Arc::new(TaskGraph::new()),
            data_scheduler: Arc::new(DataAwareScheduler::new(Arc::new(Default::default()))),
            workers: Arc::new(RwLock::new(Vec::new())),
            config,
            retry_counts: Arc::new(RwLock::new(HashMap::new())),
            reservations: Arc::new(RwLock::new(HashMap::new())),
            worker_health: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SchedulerStats::default())),
            task_queues: Arc::new(RwLock::new(HashMap::new())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            task_cache: Arc::new(RwLock::new(HashMap::new())),
            resource_predictor: Arc::new(ResourcePredictor::new()),
            scheduling_strategy: Arc::new(Box::new(DefaultSchedulingStrategy::new())),
        };

        // 启动后台任务
        scheduler.spawn_background_tasks();
        scheduler
    }

    pub async fn submit_task(&self, mut task: TaskSpec) -> Result<String> {
        let start_time = Instant::now();

        // 设置默认优先级
        if task.priority.is_none() {
            task.priority = Some(TaskPriority::Normal);
        }

        // 创建任务图节点
        let task_id = self.task_graph.add_task(
            task.function_name.clone(),
            task.args.clone(),
            vec![],
        ).await;

        // 检查是否需要抢占
        if self.should_preempt(&task).await {
            self.preempt_tasks(&task).await?;
        }

        // 尝试获取资源预留
        if let Some(reservation) = self.reserve_resources(&task).await? {
            // 有预留资源，直接调度
            let worker_id = reservation.worker_id;
            self.schedule_on_worker(task.clone(), &worker_id).await?;
        } else {
            // 无预留资源，加入队列
            let mut queues = self.task_queues.write().await;
            queues.entry(task.priority.unwrap_or(TaskPriority::Normal))
                .or_default()
                .push(task);
        }

        // 更新统计信息
        let mut stats = self.stats.write().await;
        stats.tasks_scheduled += 1;
        stats.total_scheduling_time += start_time.elapsed();

        Ok(task_id)
    }

    pub async fn register_worker(&self, worker: WorkerNode) -> Result<()> {
        // 注册到数据感知调度器
        self.data_scheduler.register_worker(worker.clone());
        
        // 添加到工作节点列表
        self.workers.write().await.push(worker.clone());
        
        // 初始化健康状态
        self.worker_health.write().await.insert(
            worker.node_info.node_id.to_string(),
            WorkerHealth {
                last_heartbeat: Instant::now(),
                failed_heartbeats: 0,
                total_tasks: 0,
                failed_tasks: 0,
                avg_task_duration: Duration::from_secs(0),
            },
        );

        // 触发任务重平衡
        self.rebalance_tasks().await?;
        
        Ok(())
    }

    pub async fn get_task_result(&self, task_id: &str) -> Result<TaskResult> {
        let task_info = self.task_graph.get_task_info(task_id).await
            .ok_or_else(|| anyhow!("Task not found"))?;

        match task_info.state {
            TaskState::Completed => {
                let mut stats = self.stats.write().await;
                stats.tasks_completed += 1;
                Ok(TaskResult::Completed(vec![]))
            }
            TaskState::Failed(error) => {
                // 检查是否需要重试
                if self.should_retry(task_id).await {
                    self.retry_task(task_id).await?;
                    Ok(TaskResult::Pending)
                } else {
                    let mut stats = self.stats.write().await;
                    stats.tasks_failed += 1;
                    Ok(TaskResult::Failed(error))
                }
            }
            TaskState::Pending | TaskState::Ready => Ok(TaskResult::Pending),
            TaskState::Running => Ok(TaskResult::Running),
        }
    }

    async fn should_preempt(&self, task: &TaskSpec) -> bool {
        if let Some(priority) = &task.priority {
            if *priority >= self.config.preemption_threshold {
                let workers = self.workers.read().await;
                for worker in workers.iter() {
                    if let Ok(running_tasks) = worker.get_running_tasks().await {
                        for running_task in running_tasks {
                            if running_task.priority < Some(self.config.preemption_threshold) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    async fn preempt_tasks(&self, new_task: &TaskSpec) -> Result<()> {
        let mut preempted = Vec::new();
        let workers = self.workers.read().await;

        for worker in workers.iter() {
            if let Ok(running_tasks) = worker.get_running_tasks().await {
                for task in running_tasks {
                    if task.priority < new_task.priority {
                        worker.stop_task(&task.task_id).await?;
                        preempted.push(task);
                    }
                }
            }
        }

        let mut stats = self.stats.write().await;
        stats.tasks_preempted += preempted.len();

        // 将被抢占的任务重新加入队列
        let mut queues = self.task_queues.write().await;
        for task in preempted {
            queues.entry(task.priority.unwrap_or(TaskPriority::Normal))
                .or_default()
                .push(task);
        }

        Ok(())
    }

    async fn reserve_resources(&self, task: &TaskSpec) -> Result<Option<ResourceReservation>> {
        let workers = self.workers.read().await;
        let mut best_worker = None;
        let mut min_load = f64::MAX;

        for worker in workers.iter() {
            if let Ok(resources) = worker.get_resources().await {
                if self.can_accommodate_task(task, &resources) {
                    let load = self.calculate_worker_load(worker).await;
                    if load < min_load {
                        min_load = load;
                        best_worker = Some(worker);
                    }
                }
            }
        }

        if let Some(worker) = best_worker {
            let reservation = ResourceReservation {
                task_id: Uuid::new_v4().to_string(),
                resources: task.required_resources.clone(),
                expires_at: Instant::now() + Duration::from_secs(60),
            };

            self.reservations.write().await
                .entry(worker.node_info.node_id.to_string())
                .or_default()
                .push(reservation.clone());

            Ok(Some(reservation))
        } else {
            Ok(None)
        }
    }

    async fn should_retry(&self, task_id: &str) -> bool {
        let retry_counts = self.retry_counts.read().await;
        let count = retry_counts.get(task_id).copied().unwrap_or(0);
        count < self.config.max_retries
    }

    async fn retry_task(&self, task_id: &str) -> Result<()> {
        // 增加重试计数
        let mut retry_counts = self.retry_counts.write().await;
        let count = retry_counts.entry(task_id.to_string()).or_insert(0);
        *count += 1;

        // 获取原始任务信息
        let task_info = self.task_graph.get_task_info(task_id).await
            .ok_or_else(|| anyhow!("Task not found"))?;

        // 创建重试任务
        let retry_task = TaskSpec {
            task_id: Uuid::new_v4().to_string(),
            function_name: task_info.function_name.clone(),
            args: task_info.args.clone(),
            priority: Some(TaskPriority::High), // 提高重试任务的优先级
            ..Default::default()
        };

        // 延迟一段时间后重试
        let delay = self.config.retry_delay;
        let retry_task_clone = retry_task.clone();
        let self_clone = Arc::new(self.clone());
        
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            if let Err(e) = self_clone.submit_task(retry_task_clone).await {
                tracing::error!("Failed to retry task {}: {}", task_id, e);
            }
        });

        Ok(())
    }

    async fn rebalance_tasks(&self) -> Result<()> {
        let workers = self.workers.read().await;
        let mut task_moves = Vec::new();

        // 计算每个工作节点的负载
        let mut worker_loads: Vec<(String, f64)> = Vec::new();
        for worker in workers.iter() {
            let load = self.calculate_worker_load(worker).await;
            worker_loads.push((worker.node_info.node_id.to_string(), load));
        }

        // 按负载排序
        worker_loads.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // 从负载最高的节点移动任务到负载最低的节点
        while let (Some(high), Some(low)) = (worker_loads.first(), worker_loads.last()) {
            if high.1 - low.1 < 0.1 {
                break;
            }

            // 找到可以移动的任务
            if let Some(worker) = workers.iter().find(|w| w.node_info.node_id.to_string() == high.0) {
                if let Ok(tasks) = worker.get_running_tasks().await {
                    if let Some(task) = tasks.into_iter().next() {
                        task_moves.push((task, low.0.clone()));
                    }
                }
            }
        }

        // 执行任务移动
        for (task, target_worker) in task_moves {
            self.schedule_on_worker(task, &target_worker).await?;
        }

        Ok(())
    }

    fn spawn_background_tasks(&self) {
        // 健康检查
        let health_check_interval = self.config.health_check_interval;
        let self_clone = Arc::new(self.clone());
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(health_check_interval).await;
                if let Err(e) = self_clone.check_worker_health().await {
                    tracing::error!("Health check failed: {}", e);
                }
            }
        });

        // 负载均衡
        let load_balance_interval = self.config.load_balance_interval;
        let self_clone = Arc::new(self.clone());
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(load_balance_interval).await;
                if let Err(e) = self_clone.rebalance_tasks().await {
                    tracing::error!("Load balancing failed: {}", e);
                }
            }
        });
    }

    async fn check_worker_health(&self) -> Result<()> {
        let workers = self.workers.read().await;
        let mut unhealthy_workers = Vec::new();

        for worker in workers.iter() {
            let worker_id = worker.node_info.node_id.to_string();
            let mut health = self.worker_health.write().await;
            
            if let Some(health) = health.get_mut(&worker_id) {
                if health.last_heartbeat.elapsed() > Duration::from_secs(30) {
                    health.failed_heartbeats += 1;
                    if health.failed_heartbeats >= 3 {
                        unhealthy_workers.push(worker_id.clone());
                    }
                }
            }
        }

        // 处理不健康的工作节点
        for worker_id in unhealthy_workers {
            self.handle_worker_failure(&worker_id).await?;
        }

        Ok(())
    }

    async fn handle_worker_failure(&self, worker_id: &str) -> Result<()> {
        // 获取失败节点上的任务
        let workers = self.workers.read().await;
        if let Some(worker) = workers.iter().find(|w| w.node_info.node_id.to_string() == worker_id) {
            if let Ok(tasks) = worker.get_running_tasks().await {
                // 重新调度这些任务
                for task in tasks {
                    self.retry_task(&task.task_id).await?;
                }
            }
        }

        // 从工作节点列表中移除
        let mut workers = self.workers.write().await;
        workers.retain(|w| w.node_info.node_id.to_string() != worker_id);

        // 清理健康状态
        self.worker_health.write().await.remove(worker_id);

        Ok(())
    }

    async fn calculate_worker_load(&self, worker: &WorkerNode) -> f64 {
        let mut load = 0.0;

        if let Ok(resources) = worker.get_resources().await {
            load += resources.cpu_usage;
            load += resources.memory_usage;
        }

        if let Ok(tasks) = worker.get_running_tasks().await {
            load += tasks.len() as f64 * 0.1;
        }

        load
    }

    fn can_accommodate_task(&self, task: &TaskSpec, resources: &WorkerResources) -> bool {
        // 检查CPU
        if let Some(cpu) = task.required_resources.cpu {
            if cpu > resources.cpu_available {
                return false;
            }
        }

        // 检查内存
        if let Some(memory) = task.required_resources.memory {
            if memory > resources.memory_available {
                return false;
            }
        }

        true
    }

    /// 提交工作流
    pub async fn submit_workflow(&self, workflow: Workflow) -> Result<String> {
        let workflow_id = workflow.id.clone();
        
        // 验证工作流
        self.validate_workflow(&workflow)?;
        
        // 存储工作流
        self.workflows.write().await.insert(workflow_id.clone(), workflow.clone());
        
        // 提交就绪的任务
        self.schedule_workflow_tasks(&workflow_id).await?;
        
        Ok(workflow_id)
    }
    
    /// 验证工作流的正确性
    fn validate_workflow(&self, workflow: &Workflow) -> Result<()> {
        // 检查是否有环
        if self.has_cycle_in_workflow(workflow) {
            return Err(anyhow!("Workflow contains cycles"));
        }
        
        // 检查所有依赖的任务是否存在
        for (task_id, deps) in &workflow.dependencies {
            if !workflow.tasks.iter().any(|t| t.id == *task_id) {
                return Err(anyhow!("Task {} not found in workflow", task_id));
            }
            for dep_id in deps {
                if !workflow.tasks.iter().any(|t| t.id == *dep_id) {
                    return Err(anyhow!("Dependency task {} not found", dep_id));
                }
            }
        }
        
        Ok(())
    }
    
    /// 检查工作流中是否存在环
    fn has_cycle_in_workflow(&self, workflow: &Workflow) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        
        for task in &workflow.tasks {
            if !visited.contains(&task.id) {
                if self.dfs_check_cycle_workflow(&task.id, workflow, &mut visited, &mut stack) {
                    return true;
                }
            }
        }
        false
    }
    
    fn dfs_check_cycle_workflow(
        &self,
        task_id: &str,
        workflow: &Workflow,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(task_id.to_string());
        stack.insert(task_id.to_string());
        
        if let Some(deps) = workflow.dependencies.get(task_id) {
            for dep_id in deps {
                if !visited.contains(dep_id) {
                    if self.dfs_check_cycle_workflow(dep_id, workflow, visited, stack) {
                        return true;
                    }
                } else if stack.contains(dep_id) {
                    return true;
                }
            }
        }
        
        stack.remove(task_id);
        false
    }
    
    /// 调度工作流中就绪的任务
    async fn schedule_workflow_tasks(&self, workflow_id: &str) -> Result<()> {
        let workflows = self.workflows.read().await;
        let workflow = workflows.get(workflow_id)
            .ok_or_else(|| anyhow!("Workflow not found"))?;
        
        // 找出所有就绪的任务
        let ready_tasks: Vec<_> = workflow.tasks.iter()
            .filter(|task| {
                let deps = workflow.dependencies.get(&task.id).cloned().unwrap_or_default();
                deps.iter().all(|dep_id| {
                    workflow.tasks.iter()
                        .find(|t| t.id == *dep_id)
                        .map(|t| t.state == TaskState::Completed)
                        .unwrap_or(false)
                })
            })
            .collect();
        
        // 提交就绪的任务
        for task in ready_tasks {
            // 检查缓存
            if let Some(cache_key) = &task.cache_key {
                if let Some(cached_result) = self.get_cached_result(cache_key).await {
                    self.update_workflow_task_state(
                        workflow_id,
                        &task.id,
                        TaskState::Completed,
                    ).await?;
                    continue;
                }
            }
            
            // 提交任务
            let mut task_spec = task.task_spec.clone();
            task_spec.workflow_id = Some(workflow_id.to_string());
            task_spec.retry_strategy = Some(task.retry_strategy.clone());
            
            self.submit_task(task_spec).await?;
        }
        
        Ok(())
    }
    
    /// 获取缓存的任务结果
    async fn get_cached_result(&self, cache_key: &str) -> Option<TaskResult> {
        let cache = self.task_cache.read().await;
        cache.get(cache_key)
            .filter(|entry| entry.created_at.elapsed() < entry.ttl)
            .map(|entry| entry.result.clone())
    }
    
    /// 更新工作流任务状态
    async fn update_workflow_task_state(
        &self,
        workflow_id: &str,
        task_id: &str,
        state: TaskState,
    ) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            if let Some(task) = workflow.tasks.iter_mut().find(|t| t.id == task_id) {
                task.state = state;
                
                // 检查工作流状态
                let all_completed = workflow.tasks.iter()
                    .all(|t| t.state == TaskState::Completed);
                let any_failed = workflow.tasks.iter()
                    .any(|t| matches!(t.state, TaskState::Failed(_)));
                
                workflow.state = if all_completed {
                    WorkflowState::Completed
                } else if any_failed {
                    WorkflowState::Failed("Some tasks failed".to_string())
                } else {
                    WorkflowState::Running
                };
                
                // 如果任务完成，调度依赖的任务
                if matches!(state, TaskState::Completed) {
                    self.schedule_workflow_tasks(workflow_id).await?;
                }
            }
        }
        Ok(())
    }
    
    /// 预测资源使用
    async fn predict_resource_usage(&self, task: &TaskSpec) -> ResourcePrediction {
        self.resource_predictor.predict_task_resources(task).await
    }
    
    /// 基于缓存位置选择工作节点
    async fn select_cache_aware_worker(&self, task: &TaskSpec) -> Option<String> {
        if let Some(cache_key) = &task.cache_key {
            let cache = self.task_cache.read().await;
            if let Some(entry) = cache.get(cache_key) {
                // 找到拥有缓存数据的节点
                let workers = self.workers.read().await;
                for worker in workers.iter() {
                    if worker.has_cached_data(cache_key).await {
                        return Some(worker.node_info.node_id.to_string());
                    }
                }
            }
        }
        None
    }
}

// 资源预测器
pub struct ResourcePredictor {
    history: Arc<RwLock<HashMap<String, Vec<ResourceUsage>>>>,
    model: Arc<RwLock<PredictionModel>>,
}

impl ResourcePredictor {
    async fn predict_task_resources(&self, task: &TaskSpec) -> ResourcePrediction {
        // 基于历史数据和机器学习模型预测资源使用
        // 这里是简化的实现
        ResourcePrediction {
            cpu_usage: vec![(Instant::now(), 0.5)],
            memory_usage: vec![(Instant::now(), 1024 * 1024)],
            network_usage: vec![(Instant::now(), (1000, 1000))],
            confidence: 0.8,
        }
    }
}

// 预测模型（示例）
struct PredictionModel {
    // 机器学习模型参数
}

#[derive(Debug, Default)]
pub struct TaskRequiredResources {
    pub cpu: Option<f64>,
    pub memory: Option<usize>,
    pub gpu: Option<usize>,
} 