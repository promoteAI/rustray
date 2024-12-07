use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::common::{TaskSpec, TaskResult, TaskRequiredResources};
use crate::error::Result;
use crate::metrics::MetricsCollector;

/// Worker node for executing tasks
pub struct WorkerNode {
    node_id: Uuid,
    address: String,
    port: u16,
    running_tasks: RwLock<HashMap<String, RunningTask>>,
    data_cache: RwLock<HashMap<String, CachedData>>,
    metrics: Arc<MetricsCollector>,
}

#[derive(Debug)]
struct RunningTask {
    spec: TaskSpec,
    start_time: Instant,
    resources: TaskRequiredResources,
}

#[derive(Debug)]
struct CachedData {
    key: String,
    size: usize,
    last_used: Instant,
}

#[derive(Debug)]
pub struct CPUMetricsResponse {
    pub usage: Vec<f32>,
    pub cores: usize,
}

#[derive(Debug)]
pub struct MemoryMetricsResponse {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage_percentage: f32,
}

#[derive(Debug)]
pub struct NetworkMetricsResponse {
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets_sent: u64,
    pub packets_recv: u64,
}

#[derive(Debug)]
pub struct StorageMetricsResponse {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage_percentage: f32,
}

impl WorkerNode {
    /// Create a new worker node
    pub fn new(address: String, port: u16, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            node_id: Uuid::new_v4(),
            address,
            port,
            running_tasks: RwLock::new(HashMap::new()),
            data_cache: RwLock::new(HashMap::new()),
            metrics,
        }
    }

    /// Submit a task for execution
    pub async fn submit_task(&self, task: TaskSpec) -> Result<TaskResult> {
        let task_id = task.task_id.to_string();
        
        // Record task start
        let running_task = RunningTask {
            spec: task.clone(),
            start_time: Instant::now(),
            resources: task.required_resources.clone(),
        };

        // Add to running tasks
        self.running_tasks.write().await.insert(task_id.clone(), running_task);

        // TODO: Implement actual task execution
        let result = TaskResult::Completed(vec![]);

        // Remove from running tasks
        self.running_tasks.write().await.remove(&task_id);

        Ok(result)
    }

    /// Get current resource usage
    pub async fn get_resource_usage(&self) -> TaskRequiredResources {
        let running = self.running_tasks.read().await;
        let mut total = TaskRequiredResources::default();

        for task in running.values() {
            if let Some(cpu) = task.resources.cpu {
                total.cpu = Some(total.cpu.unwrap_or(0.0) + cpu);
            }
            if let Some(memory) = task.resources.memory {
                total.memory = Some(total.memory.unwrap_or(0) + memory);
            }
            if let Some(gpu) = task.resources.gpu {
                total.gpu = Some(total.gpu.unwrap_or(0) + gpu);
            }
        }

        total
    }

    /// Connect to head node
    pub async fn connect_to_head(&self, _head_addr: &str) -> Result<()> {
        // TODO: Implement head node connection
        Ok(())
    }

    /// 获取集群节点数量
    pub async fn get_cluster_node_count(&self) -> usize {
        // 实现获取集群节点数量的逻辑
        // 可以通过与头节点通信或内部状态获取
        0
    }

    /// 获取正在运行的任务数量
    pub async fn get_running_tasks_count(&self) -> usize {
        let tasks = self.running_tasks.read().await;
        tasks.len()
    }

    /// 获取系统负载
    pub async fn get_system_load(&self) -> String {
        // 实现获取系统负载的逻辑
        // 可以使用 sysinfo 或其他系统信息库
        "0%".to_string()
    }

    /// 获取 CPU 指标
    pub async fn get_cpu_metrics(&self) -> CPUMetricsResponse {
        // 实现获取 CPU 指标的逻辑
        CPUMetricsResponse {
            usage: vec![0.0, 0.0, 0.0],
            cores: num_cpus::get(),
        }
    }

    /// 获取内存指标
    pub async fn get_memory_metrics(&self) -> MemoryMetricsResponse {
        // 实现获取内存指标的逻辑
        MemoryMetricsResponse {
            total: 0,
            used: 0,
            free: 0,
            usage_percentage: 0.0,
        }
    }

    /// 获取网络指标
    pub async fn get_network_metrics(&self) -> NetworkMetricsResponse {
        // 实现获取网络指标的逻辑
        NetworkMetricsResponse {
            bytes_sent: 0,
            bytes_recv: 0,
            packets_sent: 0,
            packets_recv: 0,
        }
    }

    /// 获取存储指标
    pub async fn get_storage_metrics(&self) -> StorageMetricsResponse {
        // 实现获取存储指标的逻辑
        StorageMetricsResponse {
            total: 0,
            used: 0,
            free: 0,
            usage_percentage: 0.0,
        }
    }
}
