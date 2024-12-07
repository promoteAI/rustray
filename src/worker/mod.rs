use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;
use sysinfo::{System, SystemExt, CpuExt};
use std::sync::Mutex;

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
    cached_metrics: Mutex<CachedMetrics>,
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

#[derive(Debug)]
pub struct SystemMetricsResponse {
    pub cpu: CPUMetricsResponse,
    pub memory: MemoryMetricsResponse,
    pub network: NetworkMetricsResponse,
    pub storage: StorageMetricsResponse,
}

impl WorkerNode {
    const METRICS_CACHE_DURATION: Duration = Duration::from_secs(5);
    
    /// Create a new worker node
    pub fn new(address: String, port: u16, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            node_id: Uuid::new_v4(),
            address,
            port,
            running_tasks: RwLock::new(HashMap::new()),
            data_cache: RwLock::new(HashMap::new()),
            metrics,
            cached_metrics: Mutex::new(CachedMetrics {
                cpu: CPUMetricsResponse {
                    usage: vec![],
                    cores: 0,
                },
                memory: MemoryMetricsResponse {
                    total: 0,
                    used: 0,
                    free: 0,
                    usage_percentage: 0.0,
                },
                network: NetworkMetricsResponse {
                    bytes_sent: 0,
                    bytes_recv: 0,
                    packets_sent: 0,
                    packets_recv: 0,
                },
                storage: StorageMetricsResponse {
                    total: 0,
                    used: 0,
                    free: 0,
                    usage_percentage: 0.0,
                },
                last_updated: Instant::now(),
            }),
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
        let mut system = System::new_all();
        system.refresh_all();

        // 获取 1 分钟平均负载
        let load_avg = system.load_average();
        format!("{:.2}%", load_avg.one * 100.0)
    }

    /// 获取 CPU 指标
    pub async fn get_cpu_metrics(&self) -> CPUMetricsResponse {
        let mut system = System::new_all();
        system.refresh_cpu();

        let cores = system.physical_core_count().unwrap_or(0);
        let usage = system.cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect::<Vec<f64>>();

        CPUMetricsResponse {
            usage,
            cores,
        }
    }

    /// 获取内存指标
    pub async fn get_memory_metrics(&self) -> MemoryMetricsResponse {
        let mut system = System::new_all();
        system.refresh_memory();

        let total = system.total_memory();
        let used = system.used_memory();
        let free = system.free_memory();
        let usage_percentage = (used as f64 / total as f64) * 100.0;

        MemoryMetricsResponse {
            total,
            used,
            free,
            usage_percentage,
        }
    }

    /// 获取网络指标
    pub async fn get_network_metrics(&self) -> NetworkMetricsResponse {
        let mut system = System::new_all();
        system.refresh_networks();

        let networks = system.networks();
        let (bytes_sent, bytes_recv, packets_sent, packets_recv) = networks
            .iter()
            .fold((0, 0, 0, 0), |(bs, br, ps, pr), (_, network)| {
                (
                    bs + network.get_transmitted(),
                    br + network.get_received(),
                    ps + network.get_packets_transmitted(),
                    pr + network.get_packets_received(),
                )
            });

        NetworkMetricsResponse {
            bytes_sent,
            bytes_recv,
            packets_sent,
            packets_recv,
        }
    }

    /// 获取存储指标
    pub async fn get_storage_metrics(&self) -> StorageMetricsResponse {
        let mut system = System::new_all();
        system.refresh_disks();

        // 获取根目录磁盘信息
        if let Some(disk) = system.disks().first() {
            let total = disk.total_space();
            let free = disk.available_space();
            let used = total - free;
            let usage_percentage = (used as f64 / total as f64) * 100.0;

            StorageMetricsResponse {
                total,
                used,
                free,
                usage_percentage,
            }
        } else {
            StorageMetricsResponse {
                total: 0,
                used: 0,
                free: 0,
                usage_percentage: 0.0,
            }
        }
    }

    fn get_cached_metrics(&self) -> Option<SystemMetricsResponse> {
        let cached = self.cached_metrics.lock().ok()?;
        
        if cached.last_updated.elapsed() < Self::METRICS_CACHE_DURATION {
            Some(SystemMetricsResponse {
                cpu: cached.cpu.clone(),
                memory: cached.memory.clone(),
                network: cached.network.clone(),
                storage: cached.storage.clone(),
            })
        } else {
            None
        }
    }

    async fn update_cached_metrics(&self) {
        let metrics = SystemMetricsResponse {
            cpu: self.get_cpu_metrics().await,
            memory: self.get_memory_metrics().await,
            network: self.get_network_metrics().await,
            storage: self.get_storage_metrics().await,
        };

        if let Ok(mut cached) = self.cached_metrics.lock() {
            cached.cpu = metrics.cpu;
            cached.memory = metrics.memory;
            cached.network = metrics.network;
            cached.storage = metrics.storage;
            cached.last_updated = Instant::now();
        }
    }
}
