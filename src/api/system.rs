use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, warn};

// 直接引用模块
use crate::metrics::collector::MetricsCollector;
use crate::worker::WorkerNode;
use crate::AppState;

// 系统概览响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemOverviewResponse {
    pub node_count: usize,
    pub running_tasks: usize,
    pub system_load: String,
}

// CPU 指标响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct CPUMetricsResponse {
    pub usage: Vec<f64>,
    pub cores: usize,
}

// 内存指标响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryMetricsResponse {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage_percentage: f64,
}

// 网络指标响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkMetricsResponse {
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets_sent: u64,
    pub packets_recv: u64,
}

// 存储指标响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct StorageMetricsResponse {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage_percentage: f64,
}

// 系统指标响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetricsResponse {
    pub cpu: CPUMetricsResponse,
    pub memory: MemoryMetricsResponse,
    pub network: NetworkMetricsResponse,
    pub storage: StorageMetricsResponse,
}

// 获取系统概览
pub async fn get_system_overview(
    State(metrics): State<Arc<MetricsCollector>>,
    State(worker): State<Arc<WorkerNode>>,
) -> impl IntoResponse {
    let node_count = worker.get_cluster_node_count().await;
    let running_tasks = worker.get_running_tasks_count().await;
    let system_load = worker.get_system_load().await;

    let response = SystemOverviewResponse {
        node_count,
        running_tasks,
        system_load,
    };

    (StatusCode::OK, Json(response))
}

// 获取系统指标
pub async fn get_system_metrics(
    State(state): State<AppState>
) -> Result<Json<SystemMetricsResponse>, (StatusCode, String)> {
    let worker = state.worker_node.clone();
    
    match worker.get_cached_metrics() {
        Some(cached_metrics) => Ok(Json(cached_metrics)),
        None => {
            match worker.update_cached_metrics().await {
                Ok(_) => {
                    worker.get_cached_metrics()
                        .map(Json)
                        .ok_or_else(|| {
                            error!("Failed to retrieve system metrics after update");
                            (StatusCode::INTERNAL_SERVER_ERROR, "Metrics retrieval failed".to_string())
                        })
                },
                Err(e) => {
                    error!("Error updating system metrics: {:?}", e);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, "Metrics update failed".to_string()))
                }
            }
        }
    }
}

// 获取 CPU 指标
pub async fn get_cpu_metrics(
    State(worker): State<Arc<WorkerNode>>,
) -> impl IntoResponse {
    let cpu_metrics = worker.get_cpu_metrics().await;
    (StatusCode::OK, Json(cpu_metrics))
}

// 获取内存指标
pub async fn get_memory_metrics(
    State(worker): State<Arc<WorkerNode>>,
) -> impl IntoResponse {
    let memory_metrics = worker.get_memory_metrics().await;
    (StatusCode::OK, Json(memory_metrics))
}

// 获取网络指标
pub async fn get_network_metrics(
    State(worker): State<Arc<WorkerNode>>,
) -> impl IntoResponse {
    let network_metrics = worker.get_network_metrics().await;
    (StatusCode::OK, Json(network_metrics))
}

// 获取存储指标
pub async fn get_storage_metrics(
    State(worker): State<Arc<WorkerNode>>,
) -> impl IntoResponse {
    let storage_metrics = worker.get_storage_metrics().await;
    (StatusCode::OK, Json(storage_metrics))
} 