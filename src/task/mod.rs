pub mod notification;

use crate::common::{TaskSpec, TaskResult, TaskStatus};
use crate::error::{Result, RustRayError};
use crate::notification::NotificationManager;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow;

/// Task manager responsible for managing task lifecycle
pub struct TaskManager {
    /// Task status mapping
    tasks: Arc<RwLock<HashMap<Uuid, TaskStatus>>>,
    /// Task results mapping
    results: Arc<RwLock<HashMap<Uuid, TaskResult>>>,
    /// Notification manager
    notification_manager: NotificationManager,
}

impl TaskManager {
    /// Create new task manager
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            notification_manager: NotificationManager::new(1000),
        }
    }

    /// Submit new task
    pub async fn submit_task(&self, task: TaskSpec) -> Result<Uuid> {
        let mut tasks = self.tasks.write().await;
        
        if tasks.contains_key(&task.task_id) {
            tracing::warn!("Task {} already exists", task.task_id);
            return Err(RustRayError::TaskExecutionFailed(
                format!("Task {} already exists", task.task_id)
            ));
        }

        tracing::info!("Submitting task {}", task.task_id);
        tasks.insert(task.task_id, TaskStatus::Pending);
        Ok(task.task_id)
    }

    /// Update task status
    pub async fn update_task_status(&self, task_id: Uuid, status: TaskStatus) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        
        if !tasks.contains_key(&task_id) {
            tracing::error!("Task {} not found", task_id);
            return Err(RustRayError::TaskExecutionFailed(
                format!("Task {} not found", task_id)
            ));
        }

        tracing::info!("Updating task {} status to {:?}", task_id, status);
        tasks.insert(task_id, status);
        Ok(())
    }

    /// Set task result
    pub async fn set_task_result(&self, task_id: Uuid, result: TaskResult) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        let mut results = self.results.write().await;
        
        if !tasks.contains_key(&task_id) {
            tracing::error!("Task {} not found", task_id);
            return Err(RustRayError::TaskExecutionFailed(
                format!("Task {} not found", task_id)
            ));
        }

        tracing::info!("Setting result for task {}", task_id);
        tasks.insert(task_id, TaskStatus::Completed);
        results.insert(task_id, result.clone());
        
        // Send task completion notification
        self.notification_manager.notify(result).await
            .map_err(|e| RustRayError::TaskExecutionFailed(e.to_string()))?;
        Ok(())
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: Uuid) -> Result<TaskStatus> {
        let tasks = self.tasks.read().await;
        
        tasks.get(&task_id)
            .copied()
            .ok_or_else(|| RustRayError::TaskExecutionFailed(
                format!("Task {} not found", task_id)
            ))
    }

    /// Get task result
    pub async fn get_task_result(&self, task_id: Uuid) -> Result<Option<TaskResult>> {
        let results = self.results.read().await;
        Ok(results.get(&task_id).cloned())
    }

    /// Get reference to notification manager
    pub fn notification_manager(&self) -> &NotificationManager {
        &self.notification_manager
    }
} 