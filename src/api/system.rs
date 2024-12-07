use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

// 系统概览响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemOverviewResponse {
    pub node_count: usize,
    pub running_tasks: usize,
    pub system_load: String,
}

// 系统状态响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    pub status: String,
    pub uptime: u64,
    pub node_type: String,
    pub node_id: String,
    pub version: String,
}

// 获取系统概览
pub async fn get_system_overview(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let worker = state.worker.clone();
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
    State(state): State<AppState>,
) -> impl IntoResponse {
    let worker = state.worker.clone();
    let metrics = worker.get_system_metrics().await;
    (StatusCode::OK, Json(metrics))
}

// 获取 CPU 指标
pub async fn get_cpu_metrics(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let worker = state.worker.clone();
    let cpu_metrics = worker.get_cpu_metrics().await;
    (StatusCode::OK, Json(cpu_metrics))
}

// 获取内存指标
pub async fn get_memory_metrics(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let worker = state.worker.clone();
    let memory_metrics = worker.get_memory_metrics().await;
    (StatusCode::OK, Json(memory_metrics))
}

// 获取网络指标
pub async fn get_network_metrics(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let worker = state.worker.clone();
    let network_metrics = worker.get_network_metrics().await;
    (StatusCode::OK, Json(network_metrics))
}

// 获取存储指标
pub async fn get_storage_metrics(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let worker = state.worker.clone();
    let storage_metrics = worker.get_storage_metrics().await;
    (StatusCode::OK, Json(storage_metrics))
}

// 获取系统状态
pub async fn get_system_status(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let worker = state.worker.clone();
    let node_id = worker.get_node_id().to_string();
    let uptime = worker.get_uptime().await;
    
    let response = SystemStatusResponse {
        status: "running".to_string(),
        uptime,
        node_type: "worker".to_string(),
        node_id,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    (StatusCode::OK, Json(response))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskStatusResponse {
    pub tasks: Vec<TaskInfo>,
    pub total_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub status: String,
    pub progress: f32,
    pub created_at: String,
}

// 获取任务状态
pub async fn get_task_status(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let worker = state.worker.clone();
    let tasks = worker.get_running_tasks().await;
    
    let task_infos: Vec<TaskInfo> = tasks.into_iter().map(|task| {
        TaskInfo {
            id: task.id,
            status: task.status,
            progress: task.progress,
            created_at: task.created_at,
        }
    }).collect();

    let total_count = task_infos.len();
    let response = TaskStatusResponse {
        tasks: task_infos,
        total_count,
    };

    (StatusCode::OK, Json(response))
}