use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use uuid::Uuid;
use std::collections::HashMap;
use std::time::Instant;

use crate::common::{NodeInfo, NodeType, TaskSpec, TaskResult, TaskRequiredResources, WorkerResources};

#[derive(Debug, Clone)]
pub struct WorkerNode {
    pub node_info: NodeInfo,
    running_tasks: Arc<RwLock<HashMap<String, RunningTask>>>,
    resources: Arc<RwLock<WorkerResources>>,
    cached_data: Arc<RwLock<HashMap<String, CachedData>>>,
}

#[derive(Debug)]
struct RunningTask {
    spec: TaskSpec,
    start_time: Instant,
    resources: TaskRequiredResources,
}

#[derive(Debug)]
struct CachedData {
    key: String,
    size: usize,
    last_used: Instant,
}

impl WorkerNode {
    pub fn new(address: String, port: u16) -> Self {
        Self {
            node_info: NodeInfo {
                node_id: Uuid::new_v4(),
                node_type: NodeType::Worker,
                address,
                port,
            },
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(RwLock::new(WorkerResources::default())),
            cached_data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn submit_task(&self, task: TaskSpec) -> Result<()> {
        let mut tasks = self.running_tasks.write().await;
        tasks.insert(task.task_id.to_string(), RunningTask {
            spec: task,
            start_time: Instant::now(),
            resources: TaskRequiredResources::default(),
        });
        Ok(())
    }

    pub async fn get_running_tasks(&self) -> Result<Vec<TaskSpec>> {
        let tasks = self.running_tasks.read().await;
        Ok(tasks.values().map(|t| t.spec.clone()).collect())
    }

    pub async fn stop_task(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.running_tasks.write().await;
        tasks.remove(task_id);
        Ok(())
    }

    pub async fn get_resources(&self) -> Result<WorkerResources> {
        Ok(self.resources.read().await.clone())
    }

    pub async fn update_resources(&self, resources: WorkerResources) -> Result<()> {
        *self.resources.write().await = resources;
        Ok(())
    }

    pub async fn has_cached_data(&self, key: &str) -> bool {
        let cache = self.cached_data.read().await;
        cache.contains_key(key)
    }

    pub async fn add_cached_data(&self, key: String, size: usize) -> Result<()> {
        let mut cache = self.cached_data.write().await;
        cache.insert(key.clone(), CachedData {
            key,
            size,
            last_used: Instant::now(),
        });
        Ok(())
    }

    pub async fn remove_cached_data(&self, key: &str) -> Result<()> {
        let mut cache = self.cached_data.write().await;
        cache.remove(key);
        Ok(())
    }

    pub async fn get_task_result(&self, task_id: &str) -> Result<TaskResult> {
        let tasks = self.running_tasks.read().await;
        if tasks.contains_key(task_id) {
            Ok(TaskResult::Running)
        } else {
            // 这里应该查询实际的任务结果
            Ok(TaskResult::Completed(vec![]))
        }
    }
}
