use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use sysinfo::{System, SystemExt, CpuExt, DiskExt, NetworkExt};
use crate::metrics::MetricsCollector;
use crate::error::Result;

pub mod metrics;
use metrics::{SystemMetricsResponse, CPUMetricsResponse, MemoryMetricsResponse, NetworkMetricsResponse, StorageMetricsResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningTask {
    pub id: String,
    pub status: String,
    pub progress: f32,
    pub created_at: String,
}

#[derive(Debug)]
pub struct WorkerNode {
    node_id: String,
    address: String,
    port: u16,
    running_tasks: RwLock<HashMap<String, RunningTask>>,
    metrics: Arc<MetricsCollector>,
    start_time: Instant,
}

impl WorkerNode {
    pub fn new(address: String, port: u16, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            address,
            port,
            running_tasks: RwLock::new(HashMap::new()),
            metrics,
            start_time: Instant::now(),
        }
    }

    pub fn get_node_id(&self) -> &str {
        &self.node_id
    }

    pub async fn get_uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub async fn get_running_tasks(&self) -> Vec<RunningTask> {
        let tasks = self.running_tasks.read().await;
        tasks.values().cloned().collect()
    }

    pub async fn get_running_tasks_count(&self) -> usize {
        self.running_tasks.read().await.len()
    }

    pub async fn get_cluster_node_count(&self) -> usize {
        1 // 目前只返回1，因为我们只有一个worker节点
    }

    pub async fn get_system_load(&self) -> String {
        match sys_info::loadavg() {
            Ok(load) => format!("{:.2}, {:.2}, {:.2}", load.one, load.five, load.fifteen),
            Err(_) => "N/A".to_string(),
        }
    }

    // 添加一些模拟任务用于测试
    pub async fn add_test_tasks(&self) {
        let mut tasks = self.running_tasks.write().await;
        for i in 1..=5 {
            let task_id = format!("task-{}", i);
            tasks.insert(task_id.clone(), RunningTask {
                id: task_id,
                status: "running".to_string(),
                progress: (i as f32) * 20.0,
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
            });
        }
    }

    pub async fn get_cpu_metrics(&self) -> CPUMetricsResponse {
        let mut sys = System::new();
        sys.refresh_cpu();

        let cores = sys.physical_core_count().unwrap_or(0);
        let usage = sys.cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage() as f64)
            .collect::<Vec<f64>>();

        CPUMetricsResponse {
            usage,
            cores,
        }
    }

    pub async fn get_memory_metrics(&self) -> MemoryMetricsResponse {
        let mut sys = System::new();
        sys.refresh_memory();

        let total = sys.total_memory();
        let used = sys.used_memory();
        let free = sys.free_memory();
        let usage_percentage = (used as f64 / total as f64) * 100.0;

        MemoryMetricsResponse {
            total,
            used,
            free,
            usage_percentage,
        }
    }

    pub async fn get_network_metrics(&self) -> NetworkMetricsResponse {
        let mut sys = System::new();
        sys.refresh_networks();

        let mut bytes_sent = 0;
        let mut bytes_recv = 0;
        let mut packets_sent = 0;
        let mut packets_recv = 0;

        for (_interface_name, data) in sys.networks() {
            bytes_sent += data.transmitted();
            bytes_recv += data.received();
            packets_sent += data.packets_transmitted();
            packets_recv += data.packets_received();
        }

        NetworkMetricsResponse {
            bytes_sent,
            bytes_recv,
            packets_sent,
            packets_recv,
        }
    }

    pub async fn get_storage_metrics(&self) -> StorageMetricsResponse {
        let mut sys = System::new();
        sys.refresh_disks();

        let mut total = 0;
        let mut free = 0;

        for disk in sys.disks() {
            total += disk.total_space();
            free += disk.available_space();
        }
        let used = total - free;

        let usage_percentage = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        StorageMetricsResponse {
            total,
            used,
            free,
            usage_percentage,
        }
    }

    pub async fn get_system_metrics(&self) -> SystemMetricsResponse {
        let cpu = self.get_cpu_metrics().await;
        let memory = self.get_memory_metrics().await;
        let network = self.get_network_metrics().await;
        let storage = self.get_storage_metrics().await;

        SystemMetricsResponse {
            cpu,
            memory,
            network,
            storage,
        }
    }

    pub async fn start(&self) -> Result<()> {
        // 实现启动逻辑
        Ok(())
    }
}
