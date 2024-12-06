use tokio::sync::mpsc;
use crate::common::TaskResult;
use std::sync::Arc;
use anyhow::Result;

pub struct NotificationManager {
    sender: mpsc::Sender<TaskResult>,
    receiver: mpsc::Receiver<TaskResult>,
    capacity: usize,
}

impl NotificationManager {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver,
            capacity,
        }
    }

    pub async fn notify(&self, result: TaskResult) -> Result<()> {
        self.sender.send(result).await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Option<TaskResult> {
        self.receiver.recv().await
    }
}
