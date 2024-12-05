mod common;
mod grpc;
mod head;
mod worker;
mod scheduler;
mod security;
mod task;
mod connection;
mod error;

use head::HeadNode;
use worker::WorkerNode;
use crate::scheduler::load_balancer::{LoadBalancer, LoadBalancingStrategy};
use crate::security::auth::AuthManager;
use crate::task::notification::NotificationManager;
use tokio::sync::broadcast;
use crate::connection::ConnectionManager;
use crate::error::Result;
use crate::common::TaskResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;

mod proto {
    tonic::include_proto!("rustray");
}

use proto::rustray_server::RustRayServer;
use crate::grpc::service::RustRayService;
use crate::common::object_store::ObjectStore;
use crate::scheduler::TaskScheduler;

/// 应用程序入口点
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 设置日志记录
    tracing_subscriber::fmt::init();
    tracing::info!("Starting RustRay distributed system...");

    // 初始化认证管理器
    let auth_manager = AuthManager::new(b"your-secret-key");
    tracing::info!("Authentication manager initialized");
    
    // 初始化负载均衡器，使用最小负载策略
    let _load_balancer = LoadBalancer::new(LoadBalancingStrategy::LeastLoaded);
    tracing::info!("Load balancer initialized with LeastLoaded strategy");
    
    // 初始化通知管理器，设置缓冲区大小为1000
    let notification_manager = NotificationManager::new(1000);
    tracing::info!("Notification manager initialized with buffer size 1000");

    // 创建头节点（主节点）
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    let head_clone = head.clone();
    tracing::info!("Head node created at {}:{}", head.node_info.address, head.node_info.port);
    
    // 创建工作节点
    let (worker, _task_rx) = WorkerNode::new("127.0.0.1".to_string(), 8001);
    let worker_clone = worker.clone();
    tracing::info!("Worker node created at {}:{}", worker.node_info.address, worker.node_info.port);
    
    // 为工作节点生成认证令牌
    let _worker_token = auth_manager.generate_token(&worker.node_info.node_id.to_string(), "worker")?;
    tracing::info!("Worker authentication token generated");
    
    // 启动连接管理器
    let conn_manager = ConnectionManager::new(
        format!("{}:{}", head.node_info.address, head.node_info.port),
        worker.node_info.node_id.to_string(),
    );
    tracing::info!("Connection manager initialized");

    // 创建共享组件
    let object_store = Arc::new(ObjectStore::new());
    let scheduler = Arc::new(TaskScheduler::new());

    // 创建 gRPC 服务
    let service = RustRayService::new(
        object_store.clone(),
        scheduler.clone(),
    );

    // 启动 gRPC 服务器
    let addr = "[::1]:8000".parse()?;
    println!("RustRay server listening on {}", addr);

    Server::builder()
        .add_service(RustRayServer::new(service))
        .serve(addr)
        .await?;

    // 启动所有服务并等待终止信号
    tracing::info!("Starting all services...");
    tokio::select! {
        res = head_clone.start_server() => {
            tracing::error!("Head server stopped unexpectedly: {:?}", res);
            res?;
        }
        res = worker_clone.start_server() => {
            tracing::error!("Worker server stopped unexpectedly: {:?}", res);
            res?;
        }
        res = conn_manager.maintain_connection() => {
            tracing::error!("Connection manager stopped unexpectedly: {:?}", res);
            res?;
        }
        res = handle_notifications(notification_manager.subscribe()) => {
            tracing::error!("Notification handler stopped unexpectedly: {:?}", res);
            res?;
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received shutdown signal, initiating graceful shutdown...");
        }
    }
    
    tracing::info!("System shutdown completed");
    Ok(())
}

/// 处理任务完成通知
/// 
/// # Arguments
/// * `rx` - 通知接收器
async fn handle_notifications(mut rx: broadcast::Receiver<TaskResult>) -> Result<()> {
    while let Ok(result) = rx.recv().await {
        tracing::info!(
            "Task {} completed with status: {:?}",
            result.task_id,
            result.error
        );
    }
    Ok(())
} 