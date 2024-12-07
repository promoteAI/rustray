use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::error;

use crate::common::{TaskSpec, TaskStatus};
use crate::error::{Result, RustRayError};
use crate::metrics::MetricsCollector;

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

    pub async fn add_task(&self, task: TaskSpec, deps: Vec<Uuid>) -> Result<()> {
        let task_id = task.task_id;
        let mut tasks = self.tasks.write().await;
        let mut dependencies = self.dependencies.write().await;
        let mut status = self.status.write().await;

        tasks.insert(task_id, task);
        dependencies.insert(task_id, deps.into_iter().collect());
        status.insert(task_id, TaskStatus::Pending);

        self.metrics.increment_counter("task_graph.tasks.added", 1).await;

        Ok(())
    }

    pub async fn get_ready_tasks(&self) -> Result<Vec<TaskSpec>> {
        let tasks = self.tasks.read().await;
        let dependencies = self.dependencies.read().await;
        let status = self.status.read().await;

        let mut ready_tasks = Vec::new();

        for (task_id, task) in tasks.iter() {
            if status.get(task_id) == Some(&TaskStatus::Pending) {
                let deps = dependencies.get(task_id)
                    .map(|d| d.clone())
                    .unwrap_or_else(HashSet::new);

                let all_deps_completed = deps.iter().all(|dep_id| {
                    status.get(dep_id) == Some(&TaskStatus::Completed)
                });

                if all_deps_completed {
                    ready_tasks.push(task.clone());
                }
            }
        }

        self.metrics.set_metric("task_graph.ready_tasks".to_string(), ready_tasks.len() as f64).await;

        Ok(ready_tasks)
    }

    pub async fn update_task_status(&self, task_id: Uuid, new_status: TaskStatus) -> Result<()> {
        let mut status = self.status.write().await;

        if let Some(current_status) = status.get_mut(&task_id) {
            *current_status = new_status;

            self.metrics.increment_counter(
                &format!("task_graph.status.{:?}", new_status),
                1
            ).await;

            Ok(())
        } else {
            Err(RustRayError::TaskError(format!("Task {} not found", task_id)))
        }
    }

    pub async fn get_task_status(&self, task_id: Uuid) -> Result<TaskStatus> {
        let status = self.status.read().await;

        status.get(&task_id)
            .cloned()
            .ok_or_else(|| RustRayError::TaskError(format!("Task {} not found", task_id)))
    }
} 