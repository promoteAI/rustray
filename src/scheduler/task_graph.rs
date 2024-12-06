use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use anyhow::Result;

use crate::common::{TaskSpec, TaskStatus};
use crate::metrics::collector::MetricsCollector;

pub struct TaskGraph {
    tasks: Arc<RwLock<HashMap<Uuid, TaskSpec>>>,
    dependencies: Arc<RwLock<HashMap<Uuid, HashSet<Uuid>>>>,
    status: Arc<RwLock<HashMap<Uuid, TaskStatus>>>,
    metrics: Arc<MetricsCollector>,
}

impl TaskGraph {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
            metrics,
        }
    }

    pub fn add_task(&self, task: TaskSpec, deps: Vec<Uuid>) -> Result<()> {
        let mut tasks = self.tasks.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        let mut dependencies = self.dependencies.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        let mut status = self.status.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        tasks.insert(task.task_id, task);
        dependencies.insert(task.task_id, deps.into_iter().collect());
        status.insert(task.task_id, TaskStatus::Pending);

        Ok(())
    }

    pub fn get_ready_tasks(&self) -> Result<Vec<TaskSpec>> {
        let tasks = self.tasks.read().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        let dependencies = self.dependencies.read().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        let status = self.status.read().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let ready_tasks = tasks.iter()
            .filter(|(id, _)| {
                // Task is pending and all dependencies are completed
                matches!(status.get(id), Some(TaskStatus::Pending)) &&
                dependencies.get(id)
                    .map(|deps| deps.iter().all(|dep_id| 
                        matches!(status.get(dep_id), Some(TaskStatus::Completed))
                    ))
                    .unwrap_or(true)
            })
            .map(|(_, task)| task.clone())
            .collect();

        Ok(ready_tasks)
    }

    pub fn update_task_status(&self, task_id: Uuid, new_status: TaskStatus) -> Result<()> {
        let mut status = self.status.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        status.insert(task_id, new_status);
        Ok(())
    }

    pub fn get_task_status(&self, task_id: &Uuid) -> Result<Option<TaskStatus>> {
        let status = self.status.read().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        Ok(status.get(task_id).copied())
    }

    pub fn get_task(&self, task_id: &Uuid) -> Result<Option<TaskSpec>> {
        let tasks = self.tasks.read().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        Ok(tasks.get(task_id).cloned())
    }
} 