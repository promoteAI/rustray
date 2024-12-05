use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::{mpsc, RwLock};
use std::collections::{HashMap, VecDeque};
use tracing::{info, warn, error};
use metrics::{counter, gauge};

use crate::common::{
    NodeInfo, TaskSpec, TaskStatus, TaskResult,
    WorkerResources, TaskRequiredResources,
    object_store::{ObjectId, ObjectStore},
};

pub struct WorkerNode {
    pub node_info: NodeInfo,
    object_store: Arc<ObjectStore>,
    resources: Arc<RwLock<WorkerResources>>,
    task_queue: Arc<RwLock<VecDeque<TaskSpec>>>,
    running_tasks: Arc<RwLock<HashMap<String, TaskStatus>>>,
    task_results: Arc<RwLock<HashMap<String, TaskResult>>>,
    max_concurrent_tasks: usize,
    task_sender: mpsc::Sender<TaskSpec>,
    task_receiver: Arc<RwLock<mpsc::Receiver<TaskSpec>>>,
}

impl WorkerNode {
    pub fn new(
        node_info: NodeInfo,
        object_store: Arc<ObjectStore>,
        max_concurrent_tasks: usize,
    ) -> Self {
        let (task_sender, task_receiver) = mpsc::channel(1000);
        
        Self {
            node_info,
            object_store,
            resources: Arc::new(RwLock::new(WorkerResources::default())),
            task_queue: Arc::new(RwLock::new(VecDeque::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_results: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_tasks,
            task_sender,
            task_receiver: Arc::new(RwLock::new(task_receiver)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting worker node: {}", self.node_info.node_id);
        
        // 启动资源监控
        self.start_resource_monitor().await?;
        
        // 启动任务执行器
        self.start_task_executor().await?;
        
        // 启动健康检查
        self.start_health_check().await?;
        
        Ok(())
    }

    async fn start_resource_monitor(&self) -> Result<()> {
        let resources = self.resources.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                if let Ok(mut res) = resources.write().await {
                    // 更新CPU使用率
                    if let Ok(cpu) = sys_info::cpu_usage() {
                        res.cpu_usage = cpu;
                        res.cpu_available = res.cpu_total * (100.0 - cpu) / 100.0;
                        gauge!("worker.cpu_usage", cpu);
                    }
                    
                    // 更新内存使用率
                    if let Ok(mem) = sys_info::mem_info() {
                        let total = mem.total as f64;
                        let free = mem.free as f64;
                        let usage = (total - free) / total * 100.0;
                        res.memory_usage = usage;
                        res.memory_available = mem.free as usize;
                        gauge!("worker.memory_usage", usage);
                    }
                }
            }
        });
        
        Ok(())
    }

    async fn start_task_executor(&self) -> Result<()> {
        let task_queue = self.task_queue.clone();
        let running_tasks = self.running_tasks.clone();
        let task_results = self.task_results.clone();
        let resources = self.resources.clone();
        let max_tasks = self.max_concurrent_tasks;
        let object_store = self.object_store.clone();
        
        tokio::spawn(async move {
            loop {
                // 检查是否可以执行新任务
                let can_execute = {
                    let running = running_tasks.read().await.len();
                    running < max_tasks
                };
                
                if can_execute {
                    // 从队列中获取任务
                    if let Some(task) = task_queue.write().await.pop_front() {
                        // 检查资源是否满足要求
                        if Self::check_resources(&task.required_resources, &resources).await {
                            // 更新任务状态
                            running_tasks.write().await.insert(
                                task.task_id.clone(),
                                TaskStatus::Running,
                            );
                            
                            // 预留资源
                            Self::reserve_resources(&task.required_resources, &resources).await;
                            
                            // 执行任务
                            let task_id = task.task_id.clone();
                            let task_results = task_results.clone();
                            let resources = resources.clone();
                            let object_store = object_store.clone();
                            
                            tokio::spawn(async move {
                                let result = Self::execute_task(task, object_store).await;
                                
                                // 更新任务结果
                                task_results.write().await.insert(
                                    task_id.clone(),
                                    result,
                                );
                                
                                // 释放资源
                                Self::release_resources(&task.required_resources, &resources).await;
                                
                                counter!("worker.tasks_completed", 1);
                            });
                        } else {
                            warn!("Insufficient resources for task: {}", task.task_id);
                            task_queue.write().await.push_front(task);
                        }
                    }
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        Ok(())
    }

    async fn start_health_check(&self) -> Result<()> {
        let node_id = self.node_info.node_id.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // 执行健康检查
                if let Err(e) = Self::perform_health_check(&node_id).await {
                    error!("Health check failed for node {}: {}", node_id, e);
                }
                
                gauge!("worker.health_check", 1.0);
            }
        });
        
        Ok(())
    }

    async fn check_resources(
        required: &TaskRequiredResources,
        resources: &Arc<RwLock<WorkerResources>>,
    ) -> bool {
        let res = resources.read().await;
        
        // 检查CPU
        if let Some(cpu) = required.cpu {
            if res.cpu_available < cpu {
                return false;
            }
        }
        
        // 检查内存
        if let Some(memory) = required.memory {
            if res.memory_available < memory {
                return false;
            }
        }
        
        // 检查GPU
        if let Some(gpu) = required.gpu {
            if res.gpu_available < gpu as usize {
                return false;
            }
        }
        
        true
    }

    async fn reserve_resources(
        required: &TaskRequiredResources,
        resources: &Arc<RwLock<WorkerResources>>,
    ) {
        let mut res = resources.write().await;
        
        if let Some(cpu) = required.cpu {
            res.cpu_available -= cpu;
        }
        
        if let Some(memory) = required.memory {
            res.memory_available -= memory;
        }
        
        if let Some(gpu) = required.gpu {
            res.gpu_available -= gpu as usize;
        }
    }

    async fn release_resources(
        required: &TaskRequiredResources,
        resources: &Arc<RwLock<WorkerResources>>,
    ) {
        let mut res = resources.write().await;
        
        if let Some(cpu) = required.cpu {
            res.cpu_available += cpu;
        }
        
        if let Some(memory) = required.memory {
            res.memory_available += memory;
        }
        
        if let Some(gpu) = required.gpu {
            res.gpu_available += gpu as usize;
        }
    }

    async fn execute_task(task: TaskSpec, object_store: Arc<ObjectStore>) -> TaskResult {
        info!("Executing task: {}", task.task_id);
        
        // 获取输入数据
        let mut input_data = Vec::new();
        for input_id in &task.input_objects {
            match object_store.get(input_id).await {
                Ok((data, _)) => input_data.push(data),
                Err(e) => {
                    error!("Failed to get input object {}: {}", input_id.to_string(), e);
                    return TaskResult::Failed(format!("Input data error: {}", e));
                }
            }
        }
        
        // 执行任务逻辑
        match task.execute(input_data).await {
            Ok(output_data) => {
                // 存储输出数据
                match object_store.put(output_data, task.owner.clone(), "task_output".to_string()).await {
                    Ok(output_id) => TaskResult::Completed(output_id),
                    Err(e) => TaskResult::Failed(format!("Failed to store output: {}", e)),
                }
            }
            Err(e) => TaskResult::Failed(format!("Task execution error: {}", e)),
        }
    }

    async fn perform_health_check(node_id: &str) -> Result<()> {
        // 检查系统状态
        let loadavg = sys_info::loadavg()?;
        if loadavg.one > 0.9 {
            warn!("High system load on node {}: {}", node_id, loadavg.one);
        }
        
        // 检查磁盘空间
        let disk = sys_info::disk_info()?;
        let usage = (disk.total - disk.free) as f64 / disk.total as f64;
        if usage > 0.9 {
            warn!("Low disk space on node {}: {:.1}% used", node_id, usage * 100.0);
        }
        
        Ok(())
    }

    pub async fn submit_task(&self, task: TaskSpec) -> Result<()> {
        info!("Submitting task: {}", task.task_id);
        self.task_queue.write().await.push_back(task);
        counter!("worker.tasks_submitted", 1);
        Ok(())
    }

    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        self.running_tasks.read().await.get(task_id).cloned()
    }

    pub async fn get_task_result(&self, task_id: &str) -> Option<TaskResult> {
        self.task_results.read().await.get(task_id).cloned()
    }

    pub async fn get_resources(&self) -> WorkerResources {
        self.resources.read().await.clone()
    }
} 