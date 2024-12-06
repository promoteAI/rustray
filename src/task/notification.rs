use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tracing::{info, warn};

use crate::common::{TaskStatus, TaskResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
    Created {
        task_id: String,
        owner: String,
    },
    Started {
        task_id: String,
        worker_id: String,
    },
    Completed {
        task_id: String,
        result: TaskResult,
    },
    Failed {
        task_id: String,
        error: String,
    },
    Cancelled {
        task_id: String,
        reason: String,
    },
    StatusChanged {
        task_id: String,
        old_status: TaskStatus,
        new_status: TaskStatus,
    },
    ResourcesAllocated {
        task_id: String,
        worker_id: String,
    },
    ResourcesReleased {
        task_id: String,
        worker_id: String,
    },
}

#[derive(Debug)]
pub struct TaskNotificationSystem {
    subscribers: Arc<RwLock<HashMap<String, broadcast::Sender<TaskEvent>>>>,
    max_subscribers: usize,
    channel_size: usize,
}

impl TaskNotificationSystem {
    pub fn new(max_subscribers: usize, channel_size: usize) -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            max_subscribers,
            channel_size,
        }
    }

    pub async fn subscribe(&self, subscriber_id: String) -> Result<broadcast::Receiver<TaskEvent>, String> {
        let mut subscribers = self.subscribers.write().await;
        
        if subscribers.len() >= self.max_subscribers {
            return Err("Maximum number of subscribers reached".to_string());
        }

        let (tx, rx) = broadcast::channel(self.channel_size);
        subscribers.insert(subscriber_id.clone(), tx);
        
        info!("New subscriber registered: {}", subscriber_id);
        Ok(rx)
    }

    pub async fn unsubscribe(&self, subscriber_id: &str) {
        let mut subscribers = self.subscribers.write().await;
        if subscribers.remove(subscriber_id).is_some() {
            info!("Subscriber removed: {}", subscriber_id);
        }
    }

    pub async fn publish(&self, event: TaskEvent) {
        let subscribers = self.subscribers.read().await;
        let mut failed_subscribers = Vec::new();

        for (id, sender) in subscribers.iter() {
            if let Err(e) = sender.send(event.clone()) {
                warn!("Failed to send event to subscriber {}: {}", id, e);
                failed_subscribers.push(id.clone());
            }
        }

        // 清理失败的订阅者
        if !failed_subscribers.is_empty() {
            let mut subscribers = self.subscribers.write().await;
            for id in failed_subscribers {
                subscribers.remove(&id);
                warn!("Removed failed subscriber: {}", id);
            }
        }
    }

    pub async fn publish_batch(&self, events: Vec<TaskEvent>) {
        for event in events {
            self.publish(event).await;
        }
    }

    pub async fn get_subscriber_count(&self) -> usize {
        self.subscribers.read().await.len()
    }

    pub async fn clear_subscribers(&self) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.clear();
        info!("All subscribers cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;
    use std::time::Duration;

    #[tokio::test]
    async fn test_subscribe_and_publish() {
        let system = TaskNotificationSystem::new(10, 100);
        
        // 订阅
        let mut rx = system.subscribe("test-subscriber".to_string())
            .await
            .expect("Failed to subscribe");

        // 发布事件
        let event = TaskEvent::Created {
            task_id: "task-1".to_string(),
            owner: "test-owner".to_string(),
        };
        system.publish(event.clone()).await;

        // 接收事件
        let received = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("Timeout waiting for event")
            .expect("Failed to receive event");

        match (event, received) {
            (
                TaskEvent::Created { task_id: id1, owner: owner1 },
                TaskEvent::Created { task_id: id2, owner: owner2 }
            ) => {
                assert_eq!(id1, id2);
                assert_eq!(owner1, owner2);
            }
            _ => panic!("Event type mismatch"),
        }
    }

    #[tokio::test]
    async fn test_max_subscribers() {
        let system = TaskNotificationSystem::new(2, 100);

        // 添加两个订阅者
        assert!(system.subscribe("subscriber-1".to_string()).await.is_ok());
        assert!(system.subscribe("subscriber-2".to_string()).await.is_ok());

        // 第三个订阅者应该失败
        assert!(system.subscribe("subscriber-3".to_string()).await.is_err());
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let system = TaskNotificationSystem::new(10, 100);

        // 订阅然后取消订阅
        assert!(system.subscribe("test-subscriber".to_string()).await.is_ok());
        assert_eq!(system.get_subscriber_count().await, 1);

        system.unsubscribe("test-subscriber").await;
        assert_eq!(system.get_subscriber_count().await, 0);
    }
} 