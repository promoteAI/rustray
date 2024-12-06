use tokio::sync::mpsc;
use crate::common::TaskResult;
use std::sync::Arc;
use anyhow::Result;

pub struct NotificationManager {
    sender: mpsc::Sender<TaskResult>,
    receiver: Arc<mpsc::Receiver<TaskResult>>,
    capacity: usize,
}

impl NotificationManager {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Arc::new(receiver),
            capacity,
        }
    }

    pub async fn notify(&self, result: TaskResult) -> Result<()> {
        self.sender.send(result).await?;
        Ok(())
    }

    pub async fn receive(&self) -> Option<TaskResult> {
        self.receiver.as_ref().try_recv().ok()
    }
}
