use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use std::time::Duration;

pub mod object_store;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    pub max_attempts: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
    pub retry_on_errors: Vec<String>,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_factor: 2.0,
            retry_on_errors: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub task_id: String,
    pub function_name: String,
    pub args: Vec<Vec<u8>>,
    pub kwargs: HashMap<String, String>,
    pub priority: Option<TaskPriority>,
    pub required_resources: TaskRequiredResources,
    pub timeout: Option<Duration>,
    pub retry_strategy: Option<RetryStrategy>,
    pub workflow_id: Option<String>,
    pub cache_key: Option<String>,
}

impl Default for TaskSpec {
    fn default() -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            function_name: String::new(),
            args: Vec::new(),
            kwargs: HashMap::new(),
            priority: None,
            required_resources: TaskRequiredResources::default(),
            timeout: None,
            retry_strategy: None,
            workflow_id: None,
            cache_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequiredResources {
    pub cpu: Option<f64>,
    pub memory: Option<usize>,
    pub gpu: Option<usize>,
}

impl Default for TaskRequiredResources {
    fn default() -> Self {
        Self {
            cpu: None,
            memory: None,
            gpu: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Pending,
    Running,
    Completed(Vec<u8>),
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: Uuid,
    pub node_type: NodeType,
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Head,
    Worker,
}

#[derive(Debug, Clone, Default)]
pub struct WorkerResources {
    pub cpu_total: f64,
    pub cpu_available: f64,
    pub cpu_usage: f64,
    pub memory_total: usize,
    pub memory_available: usize,
    pub memory_usage: f64,
    pub gpu_total: usize,
    pub gpu_available: usize,
    pub network_bandwidth: f64,
}