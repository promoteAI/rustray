pub mod load_balancer;
pub mod task_graph;
pub mod data_aware_scheduler;

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use uuid::Uuid;

use crate::common::{TaskSpec, TaskResult};
use crate::worker::WorkerNode;
use self::task_graph::{TaskGraph, TaskState};
use self::data_aware_scheduler::{DataAwareScheduler, WorkerResources};

/// 全局调度器
pub struct GlobalScheduler {
    task_graph: Arc<TaskGraph>,
    data_scheduler: Arc<DataAwareScheduler>,
    workers: Arc<RwLock<Vec<WorkerNode>>>,
}

/// 本地调度器
pub struct LocalScheduler {
    worker_id: String,
    resources: WorkerResources,
    task_queue: Arc<RwLock<Vec<TaskSpec>>>,
    max_concurrency: usize,
}

impl GlobalScheduler {
    pub fn new() -> Self {
        Self {
            task_graph: Arc::new(TaskGraph::new()),
            data_scheduler: Arc::new(DataAwareScheduler::new(Arc::new(Default::default()))),
            workers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn submit_task(&self, task: TaskSpec) -> Result<String> {
        // 创建任务图节点
        let task_id = self.task_graph.add_task(
            task.function_name.clone(),
            task.args.clone(),
            vec![],  // 输出对象会在执行时创建
        ).await;

        // 选择最佳工作节点
        let worker_id = self.data_scheduler.select_worker(&task.args).await?;
        
        // 获取工作节点
        let workers = self.workers.read().await;
        let worker = workers.iter()
            .find(|w| w.node_info.node_id.to_string() == worker_id)
            .ok_or_else(|| anyhow::anyhow!("Worker not found"))?;

        // 提交任务到工作节点
        worker.submit_task(task).await?;

        Ok(task_id)
    }

    pub async fn register_worker(&self, worker: WorkerNode) -> Result<()> {
        // 注册到数据感知调度器
        self.data_scheduler.register_worker(worker.clone());
        
        // 添加到工作节点列表
        self.workers.write().await.push(worker);
        
        Ok(())
    }

    pub async fn get_task_result(&self, task_id: &str) -> Result<TaskResult> {
        // 从任务图获取任务信息
        let task_info = self.task_graph.get_task_info(task_id).await
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        // 如果任务已完成，直接返回结果
        if let TaskState::Completed = task_info.state {
            // TODO: 从对象存储获取结果
            return Ok(TaskResult::Completed(vec![]));
        }

        // 如果任务失败，返回错误
        if let TaskState::Failed(error) = task_info.state {
            return Ok(TaskResult::Failed(error));
        }

        // 否则返回当前状态
        Ok(match task_info.state {
            TaskState::Pending => TaskResult::Pending,
            TaskState::Ready => TaskResult::Pending,
            TaskState::Running => TaskResult::Running,
            _ => unreachable!(),
        })
    }
}

impl LocalScheduler {
    pub fn new(worker_id: String, resources: WorkerResources) -> Self {
        Self {
            worker_id,
            resources,
            task_queue: Arc::new(RwLock::new(Vec::new())),
            max_concurrency: num_cpus::get(),
        }
    }

    pub async fn add_task(&self, task: TaskSpec) -> Result<()> {
        self.task_queue.write().await.push(task);
        Ok(())
    }

    pub async fn get_next_task(&self) -> Option<TaskSpec> {
        let mut queue = self.task_queue.write().await;
        queue.pop()
    }

    pub async fn update_resources(&mut self, resources: WorkerResources) {
        self.resources = resources;
    }

    pub fn get_worker_id(&self) -> &str {
        &self.worker_id
    }

    pub async fn get_queue_length(&self) -> usize {
        self.task_queue.read().await.len()
    }

    pub fn get_max_concurrency(&self) -> usize {
        self.max_concurrency
    }
} 