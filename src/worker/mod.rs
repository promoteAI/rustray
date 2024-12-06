use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::common::{TaskSpec, TaskResult, TaskRequiredResources};
use crate::error::Result;
use crate::metrics::MetricsCollector;

/// Worker node for executing tasks
pub struct WorkerNode {
    node_id: Uuid,
    address: String,
    port: u16,
    running_tasks: RwLock<HashMap<String, RunningTask>>,
    data_cache: RwLock<HashMap<String, CachedData>>,
    metrics: Arc<MetricsCollector>,
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
    /// Create a new worker node
    pub fn new(address: String, port: u16, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            node_id: Uuid::new_v4(),
            address,
            port,
            running_tasks: RwLock::new(HashMap::new()),
            data_cache: RwLock::new(HashMap::new()),
            metrics,
        }
    }

    /// Submit a task for execution
    pub async fn submit_task(&self, task: TaskSpec) -> Result<TaskResult> {
        let task_id = task.task_id.to_string();
        
        // Record task start
        let running_task = RunningTask {
            spec: task.clone(),
            start_time: Instant::now(),
            resources: task.required_resources.clone(),
        };

        // Add to running tasks
        self.running_tasks.write().await.insert(task_id.clone(), running_task);

        // TODO: Implement actual task execution
        let result = TaskResult::Completed(vec![]);

        // Remove from running tasks
        self.running_tasks.write().await.remove(&task_id);

        Ok(result)
    }

    /// Get current resource usage
    pub async fn get_resource_usage(&self) -> TaskRequiredResources {
        let running = self.running_tasks.read().await;
        let mut total = TaskRequiredResources::default();

        for task in running.values() {
            if let Some(cpu) = task.resources.cpu {
                total.cpu = Some(total.cpu.unwrap_or(0.0) + cpu);
            }
            if let Some(memory) = task.resources.memory {
                total.memory = Some(total.memory.unwrap_or(0) + memory);
            }
            if let Some(gpu) = task.resources.gpu {
                total.gpu = Some(total.gpu.unwrap_or(0) + gpu);
            }
        }

        total
    }

    /// Connect to head node
    pub async fn connect_to_head(&self, _head_addr: &str) -> Result<()> {
        // TODO: Implement head node connection
        Ok(())
    }
}
