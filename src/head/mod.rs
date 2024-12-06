use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};
use metrics::{counter, gauge};

use crate::common::{
    NodeInfo, TaskSpec, TaskStatus, TaskResult,
    object_store::ObjectStore,
    scheduler::Scheduler,
};
use crate::worker::WorkerNode;
use crate::security::SecurityConfig;

#[derive(Debug)]
pub struct HeadNode {
    node_info: NodeInfo,
    scheduler: Arc<Scheduler>,
    object_store: Arc<ObjectStore>,
    workers: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    security_config: SecurityConfig,
    cluster_state: Arc<RwLock<ClusterState>>,
}

#[derive(Debug, Clone)]
struct WorkerInfo {
    node: WorkerNode,
    last_heartbeat: std::time::Instant,
    status: WorkerStatus,
    tasks: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum WorkerStatus {
    Healthy,
    Unhealthy,
    Disconnected,
}

#[derive(Debug, Clone)]
struct ClusterState {
    total_workers: usize,
    active_workers: usize,
    total_tasks: usize,
    running_tasks: usize,
    completed_tasks: usize,
    failed_tasks: usize,
    resource_usage: ResourceUsage,
}

#[derive(Debug, Clone)]
struct ResourceUsage {
    total_cpu: f64,
    used_cpu: f64,
    total_memory: usize,
    used_memory: usize,
    total_gpu: usize,
    used_gpu: usize,
}

impl HeadNode {
    pub fn new(
        node_info: NodeInfo,
        scheduler: Arc<Scheduler>,
        object_store: Arc<ObjectStore>,
        security_config: SecurityConfig,
    ) -> Self {
        Self {
            node_info,
            scheduler,
            object_store,
            workers: Arc::new(RwLock::new(HashMap::new())),
            security_config,
            cluster_state: Arc::new(RwLock::new(ClusterState {
                total_workers: 0,
                active_workers: 0,
                total_tasks: 0,
                running_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                resource_usage: ResourceUsage {
                    total_cpu: 0.0,
                    used_cpu: 0.0,
                    total_memory: 0,
                    used_memory: 0,
                    total_gpu: 0,
                    used_gpu: 0,
                },
            })),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting head node: {}", self.node_info.node_id);
        
        // 启动心跳检查
        self.start_heartbeat_checker().await?;
        
        // 启动状态监控
        self.start_state_monitor().await?;
        
        // 启动任务清理
        self.start_task_cleaner().await?;
        
        Ok(())
    }

    pub async fn register_worker(&self, worker: WorkerNode) -> Result<()> {
        let worker_id = worker.node_info.node_id.clone();
        
        let mut workers = self.workers.write().await;
        let mut cluster_state = self.cluster_state.write().await;
        
        workers.insert(worker_id.clone(), WorkerInfo {
            node: worker,
            last_heartbeat: std::time::Instant::now(),
            status: WorkerStatus::Healthy,
            tasks: HashSet::new(),
        });
        
        cluster_state.total_workers += 1;
        cluster_state.active_workers += 1;
        
        info!("Worker registered: {}", worker_id);
        counter!("head.workers_registered", 1);
        Ok(())
    }

    pub async fn submit_task(&self, task: TaskSpec) -> Result<String> {
        let task_id = task.task_id.clone();
        
        // 更新集群状态
        {
            let mut state = self.cluster_state.write().await;
            state.total_tasks += 1;
            state.running_tasks += 1;
        }
        
        // 提交任务到调度器
        self.scheduler.submit_task(task).await?;
        
        info!("Task submitted: {}", task_id);
        counter!("head.tasks_submitted", 1);
        Ok(task_id)
    }

    pub async fn get_task_status(&self, task_id: &str) -> Result<TaskStatus> {
        self.scheduler.get_task_status(task_id).await
    }

    pub async fn get_task_result(&self, task_id: &str) -> Result<Option<TaskResult>> {
        self.scheduler.get_task_result(task_id).await
    }

    async fn start_heartbeat_checker(&self) -> Result<()> {
        let workers = self.workers.clone();
        let cluster_state = self.cluster_state.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                let mut workers = workers.write().await;
                let mut state = cluster_state.write().await;
                let mut disconnected = 0;
                
                for (worker_id, info) in workers.iter_mut() {
                    let elapsed = info.last_heartbeat.elapsed();
                    
                    if elapsed > tokio::time::Duration::from_secs(30) {
                        if info.status != WorkerStatus::Disconnected {
                            warn!("Worker disconnected: {}", worker_id);
                            info.status = WorkerStatus::Disconnected;
                            disconnected += 1;
                        }
                    } else if elapsed > tokio::time::Duration::from_secs(15) {
                        if info.status == WorkerStatus::Healthy {
                            warn!("Worker unhealthy: {}", worker_id);
                            info.status = WorkerStatus::Unhealthy;
                        }
                    }
                }
                
                state.active_workers = state.total_workers - disconnected;
                gauge!("head.active_workers", state.active_workers as f64);
            }
        });
        
        Ok(())
    }

    async fn start_state_monitor(&self) -> Result<()> {
        let workers = self.workers.clone();
        let cluster_state = self.cluster_state.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                let workers = workers.read().await;
                let mut state = cluster_state.write().await;
                let mut resource_usage = ResourceUsage {
                    total_cpu: 0.0,
                    used_cpu: 0.0,
                    total_memory: 0,
                    used_memory: 0,
                    total_gpu: 0,
                    used_gpu: 0,
                };
                
                for info in workers.values() {
                    if info.status == WorkerStatus::Healthy {
                        if let Ok(resources) = info.node.get_resources().await {
                            resource_usage.total_cpu += resources.cpu_total;
                            resource_usage.used_cpu += resources.cpu_total - resources.cpu_available;
                            resource_usage.total_memory += resources.memory_total;
                            resource_usage.used_memory += resources.memory_total - resources.memory_available;
                            resource_usage.total_gpu += resources.gpu_total;
                            resource_usage.used_gpu += resources.gpu_total - resources.gpu_available;
                        }
                    }
                }
                
                state.resource_usage = resource_usage;
                
                // 更新指标
                gauge!("head.cpu_usage", resource_usage.used_cpu / resource_usage.total_cpu);
                gauge!("head.memory_usage", resource_usage.used_memory as f64 / resource_usage.total_memory as f64);
                if resource_usage.total_gpu > 0 {
                    gauge!("head.gpu_usage", resource_usage.used_gpu as f64 / resource_usage.total_gpu as f64);
                }
            }
        });
        
        Ok(())
    }

    async fn start_task_cleaner(&self) -> Result<()> {
        let workers = self.workers.clone();
        let cluster_state = self.cluster_state.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let mut workers = workers.write().await;
                let mut state = cluster_state.write().await;
                
                for info in workers.values_mut() {
                    if info.status == WorkerStatus::Disconnected {
                        // 清理断开连接的工作节点的任务
                        for task_id in info.tasks.iter() {
                            state.running_tasks -= 1;
                            state.failed_tasks += 1;
                            counter!("head.tasks_failed", 1);
                        }
                        info.tasks.clear();
                    }
                }
            }
        });
        
        Ok(())
    }

    pub async fn get_cluster_state(&self) -> ClusterState {
        self.cluster_state.read().await.clone()
    }

    pub async fn get_worker_info(&self, worker_id: &str) -> Option<WorkerInfo> {
        self.workers.read().await.get(worker_id).cloned()
    }
} 