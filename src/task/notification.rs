use crate::common::TaskResult;
use crate::error::Result;
use tokio::sync::broadcast;

/// 任务通知管理器，负责处理任务完成的通知
#[derive(Clone)]
pub struct NotificationManager {
    /// 通知发送器
    sender: broadcast::Sender<TaskResult>,
    /// 缓冲区大小
    buffer_size: usize,
}

impl NotificationManager {
    /// 创建新的通知管理器
    /// 
    /// # Arguments
    /// * `buffer_size` - 通知缓冲区大小
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        Self { 
            sender,
            buffer_size,
        }
    }

    /// 发送任务完成通知
    /// 
    /// # Arguments
    /// * `result` - 任务结果
    pub async fn notify(&self, result: TaskResult) -> Result<()> {
        let _ = self.sender.send(result);
        Ok(())
    }

    /// 订阅任务完成通知
    pub fn subscribe(&self) -> broadcast::Receiver<TaskResult> {
        self.sender.subscribe()
    }

    /// 获取当前订阅者数量
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// 获取通知缓冲区容量
    pub fn buffer_capacity(&self) -> usize {
        self.buffer_size
    }
} 