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

/// 任务优先级
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    /// 关键任务，最高优先级
    Critical,
    /// 高优先级任务
    High,
    /// 普通任务（默认）
    Normal,
    /// 低优先级任务
    Low,
    /// 后台任务，最低优先级
    Background,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// 任务重试策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    /// 最大重试次数
    pub max_attempts: usize,
    /// 初始重试延迟
    pub initial_delay: Duration,
    /// 最大重试延迟
    pub max_delay: Duration,
    /// 退避因子（每次重试后延迟时间的增长倍数）
    pub backoff_factor: f64,
    /// 可重试的错误类型列表
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

/// 任务规范
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    /// 任务唯一标识符
    pub task_id: Uuid,
    /// 要执行的函数名
    pub function_name: String,
    /// 函数参数（序列化后的字节数组）
    pub args: Vec<Vec<u8>>,
    /// 函数关键字参数
    pub kwargs: HashMap<String, String>,
    /// 任务优先级
    pub priority: Option<TaskPriority>,
    /// 所需资源
    pub required_resources: TaskRequiredResources,
    /// 执行超时时间
    pub timeout: Option<Duration>,
    /// 重试策略
    pub retry_strategy: Option<RetryStrategy>,
    /// 工作流ID（如果任务属于工作流）
    pub workflow_id: Option<String>,
    /// 缓存键（用于结果复用）
    pub cache_key: Option<String>,
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
        }
    }
}

/// 任务所需资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequiredResources {
    /// CPU核心数
    pub cpu: Option<f64>,
    /// 内存大小（字节）
    pub memory: Option<usize>,
    /// GPU数量
    pub gpu: Option<usize>,
    /// 磁盘大小（字节）
    pub disk_mb: Option<u64>,
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

/// 任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    /// 等待执行
    Pending,
    /// 正在执行
    Running,
    /// 执行完成，包含结果数据
    Completed(Vec<u8>),
    /// 执行失败，包含错误信息
    Failed(String),
}

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// 节点唯一标识符
    pub node_id: Uuid,
    /// 节点类型（头节点或工作节点
    pub node_type: NodeType,
    /// 节点地址
    pub address: String,
    /// 节点端口
    pub port: u16,
}

/// 节点类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    /// 头节点，负责调度和管理
    Head,
    /// 工作节点，负责执行任务
    Worker,
}

/// 工作节点资源信息
#[derive(Debug, Clone)]
pub struct WorkerResources {
    /// CPU总核心数
    pub cpu_total: f64,
    /// 可用CPU核心数
    pub cpu_available: f64,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 总内存大小（字节）
    pub memory_total: usize,
    /// 可用内存大小（字节）
    pub memory_available: usize,
    /// 内存使用率
    pub memory_usage: f64,
    /// GPU总数量
    pub gpu_total: usize,
    /// 可用GPU数量
    pub gpu_available: usize,
    /// 网络带宽（Mbps）
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

/// 任务执行状态
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// 等待执行
    Pending,
    /// 正在执行
    Running,
    /// 执行完成
    Completed,
    /// 执行失败
    Failed,
}

/// 矩阵类型用于计算
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matrix {
    /// 行数
    pub rows: usize,
    /// 列数
    pub cols: usize,
    /// 数据
    pub data: Vec<f64>,
}

impl Matrix {
    /// 创建矩阵
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    /// 随机生成矩阵
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