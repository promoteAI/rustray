pub mod notification;

use crate::common::{TaskSpec, TaskResult};
use crate::error::{Result, RustRayError};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use metrics::{counter, histogram};

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskStatus {
    /// 等待执行
    Pending,
    /// 正在执行
    Running,
    /// 已完成
    Completed,
    /// 执行失败
    Failed,
}

/// 任务管理器，负责管理任务的生命周期
pub struct TaskManager {
    /// 任务状态映射表
    tasks: Arc<RwLock<HashMap<Uuid, TaskStatus>>>,
    /// 任务结果映射表
    results: Arc<RwLock<HashMap<Uuid, TaskResult>>>,
    /// 通知管理器
    notification_manager: notification::NotificationManager,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            notification_manager: notification::NotificationManager::new(1000),
        }
    }

    /// 提交新任务
    /// 
    /// # Arguments
    /// * `task` - 任务规范
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

    /// 更新任务状态
    /// 
    /// # Arguments
    /// * `task_id` - 任务ID
    /// * `status` - 新状态
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

    /// 设置任务结果
    /// 
    /// # Arguments
    /// * `result` - 任务结果
    pub async fn set_task_result(&self, result: TaskResult) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        let mut results = self.results.write().await;
        
        if !tasks.contains_key(&result.task_id) {
            tracing::error!("Task {} not found", result.task_id);
            return Err(RustRayError::TaskExecutionFailed(
                format!("Task {} not found", result.task_id)
            ));
        }

        tracing::info!("Setting result for task {}", result.task_id);
        tasks.insert(result.task_id, TaskStatus::Completed);
        results.insert(result.task_id, result.clone());
        
        // 发送任务完成通知
        self.notification_manager.notify(result).await?;
        Ok(())
    }

    /// 获取任务状态
    /// 
    /// # Arguments
    /// * `task_id` - 任务ID
    pub async fn get_task_status(&self, task_id: Uuid) -> Result<TaskStatus> {
        let tasks = self.tasks.read().await;
        
        tasks.get(&task_id)
            .copied()
            .ok_or_else(|| RustRayError::TaskExecutionFailed(
                format!("Task {} not found", task_id)
            ))
    }

    /// 获取任务结果
    /// 
    /// # Arguments
    /// * `task_id` - 任务ID
    pub async fn get_task_result(&self, task_id: Uuid) -> Result<Option<TaskResult>> {
        let results = self.results.read().await;
        Ok(results.get(&task_id).cloned())
    }

    /// 获取通知管理器的引用
    pub fn notification_manager(&self) -> &notification::NotificationManager {
        &self.notification_manager
    }
} 