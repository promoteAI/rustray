use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tracing::error;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/system/status", get(get_system_status))
        .route("/api/system/metrics", get(get_system_metrics))
        .route("/api/system/metrics/cpu", get(get_cpu_metrics))
        .route("/api/system/metrics/memory", get(get_memory_metrics))
        .route("/api/system/metrics/network", get(get_network_metrics))
        .route("/api/system/metrics/storage", get(get_storage_metrics))
        .route("/api/system/info", get(get_system_info))
        .route("/api/system/tasks", get(get_running_tasks))
}

pub async fn get_system_status(
    State(state): State<AppState>,
) -> Json<Value> {
    let worker = state.worker.read().await;
    
    let node_count = worker.get_cluster_node_count().await;
    let running_tasks = worker.get_running_tasks_count().await;
    let system_load = worker.get_system_load().await;

    Json(json!({
        "status": "running",
        "node_count": node_count,
        "running_tasks": running_tasks,
        "system_load": system_load,
    }))
}

pub async fn get_system_metrics(
    State(state): State<AppState>,
) -> Json<Value> {
    let worker = state.worker.read().await;
    let metrics = worker.get_system_metrics().await;
    Json(json!(metrics))
}

pub async fn get_cpu_metrics(
    State(state): State<AppState>,
) -> Json<Value> {
    let worker = state.worker.read().await;
    let cpu_metrics = worker.get_cpu_metrics().await;
    Json(json!(cpu_metrics))
}

pub async fn get_memory_metrics(
    State(state): State<AppState>,
) -> Json<Value> {
    let worker = state.worker.read().await;
    let memory_metrics = worker.get_memory_metrics().await;
    Json(json!(memory_metrics))
}

pub async fn get_network_metrics(
    State(state): State<AppState>,
) -> Json<Value> {
    let worker = state.worker.read().await;
    let network_metrics = worker.get_network_metrics().await;
    Json(json!(network_metrics))
}

pub async fn get_storage_metrics(
    State(state): State<AppState>,
) -> Json<Value> {
    let worker = state.worker.read().await;
    let storage_metrics = worker.get_storage_metrics().await;
    Json(json!(storage_metrics))
}

pub async fn get_system_info(
    State(state): State<AppState>,
) -> Json<Value> {
    let worker = state.worker.read().await;
    let node_id = worker.get_node_id().to_string();
    let uptime = worker.get_uptime().await;

    Json(json!({
        "node_id": node_id,
        "uptime": uptime,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn get_running_tasks(
    State(state): State<AppState>,
) -> Json<Value> {
    let worker = state.worker.read().await;
    let tasks = worker.get_running_tasks().await;
    Json(json!(tasks))
}