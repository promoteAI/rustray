use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsResponse {
    pub cpu: CPUMetricsResponse,
    pub memory: MemoryMetricsResponse,
    pub network: NetworkMetricsResponse,
    pub storage: StorageMetricsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUMetricsResponse {
    pub usage: Vec<f64>,
    pub cores: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetricsResponse {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetricsResponse {
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets_sent: u64,
    pub packets_recv: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetricsResponse {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage_percentage: f64,
} 