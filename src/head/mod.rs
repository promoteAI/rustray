//! 头节点模块
//! 
//! 本模块实现了分布式系统的头节点，负责：
//! - 集群管理和协调
//! - 任务调度和分发
//! - 资源管理和分配
//! - 故障检测和恢复
//! - 性能监控和优化

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};
use uuid::Uuid;
use futures_util::future::TryFutureExt;

use crate::common::{
    NodeInfo, NodeType, TaskSpec, TaskRequiredResources, TaskResult, TaskStatus,
    object_store::ObjectStore,
};
use crate::metrics::MetricsCollector;
use crate::scheduler::{LoadBalancer, LoadBalanceStrategy, TaskGraph, NodeHealth};
use crate::error::{Result, RustRayError};

/// Head node configuration
#[derive(Debug, Clone)]
pub struct HeadNodeConfig {
    pub address: String,
    pub port: u16,
    pub heartbeat_timeout: Duration,
    pub resource_monitor_interval: Duration,
    pub scheduling_strategy: LoadBalanceStrategy,
}

/// Head node state
#[derive(Debug, Clone, PartialEq)]
pub enum HeadNodeState {
    Initializing,
    Running,
    Degraded(String),
    ShuttingDown,
    Shutdown,
    Error(String),
}

/// Cluster status
#[derive(Debug, Clone)]
pub struct ClusterStatus {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub pending_tasks: usize,
    pub cpu_usage: f64,
    pub memory_usage: usize,
}

/// Head node
pub struct HeadNode {
    pub node_info: NodeInfo,
    config: HeadNodeConfig,
    state: Arc<RwLock<HeadNodeState>>,
    workers: Arc<RwLock<HashMap<Uuid, WorkerInfo>>>,
    task_graph: Arc<TaskGraph>,
    load_balancer: Arc<LoadBalancer>,
    object_store: Arc<ObjectStore>,
    metrics: Arc<MetricsCollector>,
    status_tx: mpsc::Sender<ClusterStatus>,
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

/// Worker node information
#[derive(Debug, Clone)]
struct WorkerInfo {
    node_info: NodeInfo,
    last_heartbeat: Instant,
    resources: TaskRequiredResources,
    running_tasks: usize,
}

impl HeadNode {
    pub fn new(
        config: HeadNodeConfig,
        object_store: Arc<ObjectStore>,
        metrics: Arc<MetricsCollector>,
        status_tx: mpsc::Sender<ClusterStatus>,
    ) -> Self {
        let node_info = NodeInfo {
            node_id: Uuid::new_v4(),
            node_type: NodeType::Head,
            address: config.address.clone(),
            port: config.port,
        };

        let task_graph = Arc::new(TaskGraph::new(metrics.clone()));
        let load_balancer = Arc::new(LoadBalancer::new(
            config.scheduling_strategy.clone(),
            metrics.clone(),
        ));

        Self {
            node_info,
            config,
            state: Arc::new(RwLock::new(HeadNodeState::Initializing)),
            workers: Arc::new(RwLock::new(HashMap::new())),
            task_graph,
            load_balancer,
            object_store,
            metrics,
            status_tx,
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting head node: {}", self.node_info.node_id);

        self.start_heartbeat_checker().await?;
        self.start_resource_monitor().await?;
        self.start_scheduler().await?;

        *self.state.write().await = HeadNodeState::Running;
        
        self.update_status().await?;
        info!("Head node started successfully");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        info!("Stopping head node: {}", self.node_info.node_id);

        *self.state.write().await = HeadNodeState::ShuttingDown;

        let mut tasks = self.background_tasks.write().await;
        for task in tasks.drain(..) {
            task.abort();
        }

        *self.state.write().await = HeadNodeState::Shutdown;
        
        self.update_status().await?;
        info!("Head node stopped successfully");
        Ok(())
    }

    pub async fn register_worker(&self, worker: NodeInfo) -> Result<()> {
        info!("Registering worker node: {}", worker.node_id);

        let worker_info = WorkerInfo {
            node_info: worker.clone(),
            last_heartbeat: Instant::now(),
            resources: TaskRequiredResources::default(),
            running_tasks: 0,
        };

        self.workers.write().await
            .insert(worker.node_id, worker_info);

        self.load_balancer.register_node(worker).await
            .map_err(|e| RustRayError::InternalError(e.to_string()))?;

        let metrics = &*self.metrics;
        metrics.increment_counter("head.workers.registered", 1).await
            .map_err(|e| RustRayError::InternalError(e.to_string()))?;

        self.update_status().await?;
        info!("Worker node registered successfully");
        Ok(())
    }

    pub async fn submit_task(&self, task: TaskSpec) -> Result<()> {
        info!("Submitting task: {}", task.task_id);

        self.task_graph.add_task(task.clone(), vec![]).await
            .map_err(|e| RustRayError::TaskError(e.to_string()))?;

        let worker_id = self.load_balancer.select_node(&task).await?;
        
        if let Some(worker_id) = worker_id {
            let workers = self.workers.read().await;
                
            if let Some(worker) = workers.get(&worker_id) {
                // TODO: Implement task sending logic
                
                let metrics = &*self.metrics;
                metrics.increment_counter("head.tasks.submitted", 1).await
                    .map_err(|e| RustRayError::InternalError(e.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn update_worker_heartbeat(
        &self,
        worker_id: Uuid,
        resources: TaskRequiredResources,
    ) -> Result<()> {
        let mut workers = self.workers.write().await;
        
        if let Some(worker) = workers.get_mut(&worker_id) {
            worker.last_heartbeat = Instant::now();
            worker.resources = resources;

            self.load_balancer.update_node_health(worker_id, NodeHealth::Healthy).await
                .map_err(|e| RustRayError::InternalError(e.to_string()))?;

            self.metrics.increment_counter("head.heartbeats.received", 1).await
                .map_err(|e| RustRayError::InternalError(e.to_string()))?;
        }

        self.update_status().await?;
        Ok(())
    }

    async fn update_status(&self) -> Result<()> {
        let workers = self.workers.read().await;
        
        let status = ClusterStatus {
            total_nodes: workers.len(),
            active_nodes: workers.iter()
                .filter(|(_, w)| w.last_heartbeat.elapsed() < self.config.heartbeat_timeout)
                .count(),
            total_tasks: 0, // TODO: Implement
            running_tasks: workers.iter().map(|(_, w)| w.running_tasks).sum(),
            pending_tasks: 0, // TODO: Implement
            cpu_usage: 0.0, // TODO: Implement
            memory_usage: 0, // TODO: Implement
        };

        self.status_tx.send(status).await
            .map_err(|e| RustRayError::CommunicationError(e.to_string()))?;
        Ok(())
    }

    async fn start_heartbeat_checker(&self) -> Result<()> {
        let state = self.state.clone();
        let workers = self.workers.clone();
        let load_balancer = self.load_balancer.clone();
        let metrics = self.metrics.clone();
        let timeout = self.config.heartbeat_timeout;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(timeout);
            loop {
                interval.tick().await;
                
                if *state.read().await == HeadNodeState::ShuttingDown {
                    break;
                }

                let mut workers = workers.write().await;
                let now = Instant::now();
                let mut to_remove = Vec::new();

                for (worker_id, info) in workers.iter() {
                    if now.duration_since(info.last_heartbeat) >= timeout {
                        to_remove.push(*worker_id);
                    }
                }

                for worker_id in to_remove {
                    workers.remove(&worker_id);
                    match load_balancer.update_node_health(
                        worker_id,
                        NodeHealth::Unhealthy("Heartbeat timeout".to_string()),
                    ).await {
                        Ok(_) => {
                            if let Err(e) = metrics.increment_counter("head.workers.timeout", 1).await {
                                error!("Failed to update metrics: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to update node health: {}", e);
                        }
                    }
                }
            }
        });

        self.background_tasks.write().await.push(handle);
        Ok(())
    }

    async fn start_resource_monitor(&self) -> Result<()> {
        let state = self.state.clone();
        let workers = self.workers.clone();
        let metrics = self.metrics.clone();
        let interval = self.config.resource_monitor_interval;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                
                if *state.read().await == HeadNodeState::ShuttingDown {
                    break;
                }

                let workers = workers.read().await;
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

                if active_workers > 0 {
                    let avg_cpu = total_cpu / active_workers as f64;
                    let avg_memory = total_memory / active_workers;

                    if let Err(e) = metrics.set_gauge("head.cluster.cpu_usage", avg_cpu).await {
                        error!("Failed to update CPU metrics: {}", e);
                    }
                    if let Err(e) = metrics.set_gauge("head.cluster.memory_usage", avg_memory as f64).await {
                        error!("Failed to update memory metrics: {}", e);
                    }
                }
            }
        });

        self.background_tasks.write().await.push(handle);
        Ok(())
    }

    async fn start_scheduler(&self) -> Result<()> {
        let state = self.state.clone();
        let task_graph = self.task_graph.clone();
        let load_balancer = self.load_balancer.clone();
        let metrics = self.metrics.clone();

        let handle = tokio::spawn(async move {
            loop {
                if *state.read().await == HeadNodeState::ShuttingDown {
                    break;
                }

                // Get next ready task
                match task_graph.get_ready_tasks().await {
                    Ok(ready_tasks) => {
                        for task in ready_tasks {
                            match load_balancer.select_node(&task).await {
                                Ok(Some(worker_id)) => {
                                    // TODO: Implement task dispatch logic
                                    if let Err(e) = metrics.increment_counter("head.tasks.scheduled", 1).await {
                                        error!("Failed to update metrics: {}", e);
                                    }
                                }
                                Ok(None) => {
                                    info!("No available worker for task {}", task.task_id);
                                }
                                Err(e) => {
                                    error!("Failed to select node for task {}: {}", task.task_id, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get ready tasks: {}", e);
                    }
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        self.background_tasks.write().await.push(handle);
        Ok(())
    }

    async fn check_worker_health(&self) -> Result<()> {
        let workers = self.workers.read().await;
        let metrics = &*self.metrics;

        for (worker_id, worker) in workers.iter() {
            if worker.last_heartbeat.elapsed() > self.config.heartbeat_timeout {
                match self.load_balancer.update_node_health(
                    *worker_id,
                    NodeHealth::Unhealthy("Heartbeat timeout".to_string())
                ).await {
                    Ok(_) => {
                        if let Err(e) = metrics.increment_counter("head.workers.unhealthy", 1).await {
                            error!("Failed to update metrics: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to update node health: {}", e);
                        return Err(RustRayError::InternalError(e.to_string()));
                    }
                }
            }
        }

        Ok(())
    }

    async fn update_cluster_metrics(&self) -> Result<()> {
        let workers = self.workers.read().await;
        let metrics = &*self.metrics;

        let mut total_cpu_usage = 0.0;
        let mut total_memory_usage = 0;

        for worker in workers.values() {
            if let Some(cpu) = worker.resources.cpu {
                total_cpu_usage += cpu;
            }
            if let Some(memory) = worker.resources.memory {
                total_memory_usage += memory;
            }
        }

        metrics.set_gauge("head.cluster.cpu_usage", total_cpu_usage).await
            .map_err(|e| RustRayError::InternalError(e.to_string()))?;

        metrics.set_gauge("head.cluster.memory_usage", total_memory_usage as f64).await
            .map_err(|e| RustRayError::InternalError(e.to_string()))?;

        Ok(())
    }

    pub async fn get_task_result(&self, _task_id: &Uuid) -> Result<Option<TaskResult>> {
        // TODO: Implement task result retrieval
        Ok(None)
    }

    pub async fn get_task_status(&self, task_id: Uuid) -> Result<TaskStatus> {
        self.task_graph.get_task_status(task_id).await
    }

    async fn schedule_tasks(&self) -> Result<()> {
        let tasks = self.task_graph.get_ready_tasks().await?;
        let load_balancer = self.load_balancer.clone();
        let metrics = &*self.metrics;

        for task in tasks {
            match load_balancer.select_node(&task).await {
                Ok(Some(worker_id)) => {
                    let workers = self.workers.read().await;
                    if let Some(_worker) = workers.get(&worker_id) {
                        // TODO: Implement task scheduling logic
                        match metrics.increment_counter("head.tasks.scheduled", 1).await {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Failed to update metrics: {}", e);
                                return Err(RustRayError::InternalError(e.to_string()));
                            }
                        }
                    }
                }
                Ok(None) => {
                    info!("No available worker for task {}", task.task_id);
                }
                Err(e) => {
                    error!("Failed to select node for task {}: {}", task.task_id, e);
                    return Err(RustRayError::InternalError(e.to_string()));
                }
            }
        }

        Ok(())
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
            *head.state.read().await,
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
            node_id: Uuid::new_v4(),
            node_type: NodeType::Worker,
            address: "localhost".to_string(),
            port: 8001,
        };

        let result = head.register_worker(worker.clone()).await;
        assert!(result.is_ok());
        
        let workers = head.workers.read().await;
        assert_eq!(workers.len(), 1);
        assert!(workers.contains_key(&worker.node_id));
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
            task_id: Uuid::new_v4().to_string(),
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