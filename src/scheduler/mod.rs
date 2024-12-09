use tokio::sync::{mpsc, RwLock};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use log::{info, warn, error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskPriority {
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub priority: TaskPriority,
    pub payload: Vec<u8>,
    pub created_at: Instant,
    pub deadline: Option<Instant>,
    pub retries: u32,
}

impl Task {
    pub fn new(priority: TaskPriority, payload: Vec<u8>, deadline: Option<Duration>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            priority,
            payload,
            created_at: Instant::now(),
            deadline: deadline.map(|d| Instant::now() + d),
            retries: 0,
        }
    }
}

pub struct Scheduler {
    high_priority_queue: Arc<RwLock<BinaryHeap<Reverse<(Instant, String)>>>>,
    normal_priority_queue: Arc<RwLock<BinaryHeap<Reverse<(Instant, String)>>>>,
    low_priority_queue: Arc<RwLock<BinaryHeap<Reverse<(Instant, String)>>>>,
    tasks: Arc<RwLock<HashMap<String, Task>>>,
    task_sender: mpsc::Sender<Task>,
}

impl Scheduler {
    pub fn new(task_sender: mpsc::Sender<Task>) -> Self {
        Self {
            high_priority_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            normal_priority_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            low_priority_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_sender,
        }
    }

    pub async fn submit_task(&self, task: Task) -> Result<(), String> {
        let queue = match task.priority {
            TaskPriority::High => &self.high_priority_queue,
            TaskPriority::Normal => &self.normal_priority_queue,
            TaskPriority::Low => &self.low_priority_queue,
        };

        let deadline = task.deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(3600));
        queue.write().await.push(Reverse((deadline, task.id.clone())));
        self.tasks.write().await.insert(task.id.clone(), task);
        
        Ok(())
    }

    pub async fn run(&self) {
        loop {
            // 优先处理高优先级任务
            if let Some(task) = self.get_next_task().await {
                if let Err(e) = self.task_sender.send(task.clone()).await {
                    error!("Failed to send task: {}", e);
                    // 重试逻辑
                    if task.retries < 3 {
                        let mut retry_task = task;
                        retry_task.retries += 1;
                        self.submit_task(retry_task).await.unwrap_or_else(|e| {
                            error!("Failed to resubmit task: {}", e);
                        });
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn get_next_task(&self) -> Option<Task> {
        // 按优先级顺序检查队列
        for queue in &[
            &self.high_priority_queue,
            &self.normal_priority_queue,
            &self.low_priority_queue,
        ] {
            let mut queue = queue.write().await;
            if let Some(Reverse((deadline, task_id))) = queue.pop() {
                if Instant::now() <= deadline {
                    if let Some(task) = self.tasks.write().await.remove(&task_id) {
                        return Some(task);
                    }
                }
            }
        }
        None
    }
}

pub mod data_aware_scheduler;
pub mod load_balancer;
pub mod task_graph;

pub use self::load_balancer::LoadBalancer;
pub use self::task_graph::TaskGraph;

/// Node health status
#[derive(Debug, Clone, PartialEq)]
pub enum NodeHealth {
    Healthy,
    Unhealthy(String),
}

/// Load balancing strategy
#[derive(Debug, Clone)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastLoaded,
    DataAware,
    Custom(String),
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        Self::LeastLoaded
    }
}
  