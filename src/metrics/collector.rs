use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use sysinfo::{System, SystemExt, CpuExt, DiskExt, NetworkExt};

#[derive(Debug, Clone)]
pub struct MetricsCollector {
    system: Arc<RwLock<System>>,
    metrics: Arc<RwLock<HashMap<String, f64>>>,
    counters: Arc<RwLock<HashMap<String, i64>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        Self {
            system: Arc::new(RwLock::new(sys)),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn increment_counter(&self, key: &str, value: i64) {
        let mut counters = self.counters.write().await;
        *counters.entry(key.to_string()).or_insert(0) += value;
    }

    pub async fn get_counter(&self, key: &str) -> i64 {
        let counters = self.counters.read().await;
        *counters.get(key).unwrap_or(&0)
    }

    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), value);
    }

    pub async fn collect_cpu_metrics(&self) -> HashMap<String, f64> {
        let mut sys = self.system.write().await;
        sys.refresh_cpu();

        let mut metrics = HashMap::new();
        let cpu_count = sys.cpus().len();
        let mut total_usage = 0.0;

        for (i, cpu) in sys.cpus().iter().enumerate() {
            let usage = cpu.cpu_usage() as f64;
            metrics.insert(format!("cpu_{}_usage", i), usage);
            total_usage += usage;
        }

        metrics.insert("cpu_count".to_string(), cpu_count as f64);
        metrics.insert("cpu_average_usage".to_string(), total_usage / cpu_count as f64);

        metrics
    }

    pub async fn collect_memory_metrics(&self) -> HashMap<String, f64> {
        let mut sys = self.system.write().await;
        sys.refresh_memory();

        let mut metrics = HashMap::new();
        metrics.insert("total_memory".to_string(), sys.total_memory() as f64);
        metrics.insert("used_memory".to_string(), sys.used_memory() as f64);
        metrics.insert("total_swap".to_string(), sys.total_swap() as f64);
        metrics.insert("used_swap".to_string(), sys.used_swap() as f64);

        metrics
    }

    pub async fn collect_disk_metrics(&self) -> HashMap<String, f64> {
        let mut sys = self.system.write().await;
        sys.refresh_disks();

        let mut metrics = HashMap::new();
        let mut total_space = 0.0;
        let mut available_space = 0.0;

        for disk in sys.disks() {
            total_space += disk.total_space() as f64;
            available_space += disk.available_space() as f64;
        }

        metrics.insert("disk_total_space".to_string(), total_space);
        metrics.insert("disk_available_space".to_string(), available_space);
        metrics.insert("disk_used_space".to_string(), total_space - available_space);

        metrics
    }

    pub async fn collect_network_metrics(&self) -> HashMap<String, f64> {
        let mut sys = self.system.write().await;
        sys.refresh_networks();

        let mut metrics = HashMap::new();
        let mut total_rx = 0.0;
        let mut total_tx = 0.0;

        for (_interface, data) in sys.networks() {
            total_rx += data.received() as f64;
            total_tx += data.transmitted() as f64;
        }

        metrics.insert("network_total_received".to_string(), total_rx);
        metrics.insert("network_total_transmitted".to_string(), total_tx);

        metrics
    }

    pub async fn collect_all_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();

        // Collect CPU metrics
        metrics.extend(self.collect_cpu_metrics().await);

        // Collect memory metrics
        metrics.extend(self.collect_memory_metrics().await);

        // Collect disk metrics
        metrics.extend(self.collect_disk_metrics().await);

        // Collect network metrics
        metrics.extend(self.collect_network_metrics().await);

        metrics
    }

    pub async fn get_metric(&self, name: &str) -> Option<f64> {
        let metrics = self.metrics.read().await;
        metrics.get(name).copied()
    }

    pub async fn set_metric(&self, name: String, value: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.insert(name, value);
    }

    pub async fn get_all_metrics(&self) -> HashMap<String, f64> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
}
