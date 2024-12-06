//! 通用模块
//! 
//! 本模块包含系统中共享的核心数据结构和类型定义。
//! 主要包括：
//! - 任务相关类型（TaskSpec, TaskPriority等）
//! - 资源管理类型（TaskRequiredResources等）
//! - 节点管理类型（NodeInfo, NodeType等）
//! - 重试策略（RetryStrategy）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use std::time::Duration;
use rand;

pub mod object_store;

/// Task priority levels
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

/// Task execution status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Running,
    Completed(Vec<u8>),
    Failed(String)
}

/// Task specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub task_id: Uuid,
    pub function_name: String,
    pub args: Vec<Vec<u8>>,
    pub kwargs: HashMap<String, String>,
    pub priority: Option<TaskPriority>,
    pub required_resources: TaskRequiredResources,
    pub timeout: Option<Duration>,
    pub retry_strategy: Option<RetryStrategy>,
    pub workflow_id: Option<String>,
    pub cache_key: Option<String>,
    pub data_dependencies: Vec<String>,
}

impl Default for TaskSpec {
    fn default() -> Self {
        Self {
            task_id: Uuid::new_v4(),
            function_name: String::new(),
            args: Vec::new(),
            kwargs: HashMap::new(),
            priority: None,
            required_resources: TaskRequiredResources::default(),
            timeout: None,
            retry_strategy: None,
            workflow_id: None,
            cache_key: None,
            data_dependencies: Vec::new(),
        }
    }
}

/// Required resources for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequiredResources {
    pub cpu: Option<f64>,
    pub memory: Option<usize>,
    pub gpu: Option<usize>,
    pub disk_mb: Option<u64>,
}

impl TaskRequiredResources {
    pub fn can_fit(&self, available: &TaskRequiredResources) -> bool {
        // 如果没有指定任何资源要求，则认为可以运行
        if self.cpu.is_none() && self.memory.is_none() && 
           self.gpu.is_none() && self.disk_mb.is_none() {
            return true;
        }

        // 检查CPU要求
        if let Some(req_cpu) = self.cpu {
            match available.cpu {
                Some(avail_cpu) if req_cpu <= avail_cpu => (),
                _ => return false,
            }
        }

        // 检查内存要求
        if let Some(req_mem) = self.memory {
            match available.memory {
                Some(avail_mem) if req_mem <= avail_mem => (),
                _ => return false,
            }
        }

        // 检查GPU要求
        if let Some(req_gpu) = self.gpu {
            match available.gpu {
                Some(avail_gpu) if req_gpu <= avail_gpu => (),
                _ => return false,
            }
        }

        // 检查磁盘要求
        if let Some(req_disk) = self.disk_mb {
            match available.disk_mb {
                Some(avail_disk) if req_disk <= avail_disk => (),
                _ => return false,
            }
        }

        true
    }

    pub fn is_empty(&self) -> bool {
        self.cpu.is_none() && self.memory.is_none() && 
        self.gpu.is_none() && self.disk_mb.is_none()
    }
}

impl Default for TaskRequiredResources {
    fn default() -> Self {
        Self {
            cpu: None,
            memory: None,
            gpu: None,
            disk_mb: None,
        }
    }
}

/// Retry strategy for failed tasks
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

/// Node type (Head or Worker)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Head,
    Worker,
}

/// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: Uuid,
    pub node_type: NodeType,
    pub address: String,
    pub port: u16,
}

/// Worker node resources
#[derive(Debug, Clone)]
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

impl Default for WorkerResources {
    fn default() -> Self {
        Self {
            cpu_total: 0.0,
            cpu_available: 0.0,
            cpu_usage: 0.0,
            memory_total: 0,
            memory_available: 0,
            memory_usage: 0.0,
            gpu_total: 0,
            gpu_available: 0,
            network_bandwidth: 0.0,
        }
    }
}

/// Matrix type for computations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    pub fn random(rows: usize, cols: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let data = (0..rows * cols).map(|_| rng.gen()).collect();
        Self { rows, cols, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority_default() {
        assert_eq!(TaskPriority::default(), TaskPriority::Normal);
    }

    #[test]
    fn test_retry_strategy() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.max_attempts, 3);
        assert_eq!(strategy.initial_delay, Duration::from_secs(1));
        assert_eq!(strategy.max_delay, Duration::from_secs(60));
        assert_eq!(strategy.backoff_factor, 2.0);
    }

    #[test]
    fn test_task_spec_creation() {
        let task = TaskSpec {
            function_name: "test_function".to_string(),
            args: vec![vec![1, 2, 3]],
            priority: Some(TaskPriority::High),
            ..Default::default()
        };

        assert_eq!(task.function_name, "test_function");
        assert_eq!(task.args, vec![vec![1, 2, 3]]);
        assert_eq!(task.priority, Some(TaskPriority::High));
    }

    #[test]
    fn test_worker_resources() {
        let mut resources = WorkerResources::default();
        resources.cpu_total = 8.0;
        resources.cpu_available = 6.0;
        resources.cpu_usage = 25.0;
        
        assert_eq!(resources.cpu_total, 8.0);
        assert_eq!(resources.cpu_available, 6.0);
        assert_eq!(resources.cpu_usage, 25.0);
    }
}