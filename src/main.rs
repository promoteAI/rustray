//! RustRay Distributed Task Processing Framework

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, ValueEnum};
use tokio::{signal, sync::mpsc};
use tonic::transport::Server;
use tracing::{info, warn, error, Level};
use tracing_subscriber::FmtSubscriber;

use rustray::{
    proto::rustray_server::RustrayServer,
    grpc::RustRayService,
    head::{HeadNode, HeadNodeConfig},
    worker::WorkerNode,
    error::Result,
    common::object_store::ObjectStore,
    metrics::MetricsCollector,
    scheduler::LoadBalanceStrategy,
};

mod api;

use axum::{
    routing::get,
    Router,
};

#[derive(Clone)]
struct AppState {
    metrics: Arc<MetricsCollector>,
    worker: Arc<WorkerNode>,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

fn create_api_routes(
    metrics: Arc<MetricsCollector>,
    worker: Arc<WorkerNode>,
) -> Router {
    let app_state = AppState {
        metrics: metrics.clone(),
        worker: worker.clone(),
    };

    Router::new()
        .route("/api/system/status", get(api::system::get_system_status))
        .route("/api/system/overview", get(api::system::get_system_overview))
        .route("/api/system/metrics", get(api::system::get_system_metrics))
        .route("/api/metrics/cpu", get(api::system::get_cpu_metrics))
        .route("/api/metrics/memory", get(api::system::get_memory_metrics))
        .route("/api/metrics/network", get(api::system::get_network_metrics))
        .route("/api/metrics/storage", get(api::system::get_storage_metrics))
        .route("/api/tasks/status", get(api::system::get_task_status))
        .with_state(app_state)
}

/// Command-line arguments for RustRay nodes
#[derive(Parser, Debug)]
#[command(
    name = "RustRay",
    version = "0.1.0",
    about = "Distributed task processing framework",
    long_about = "A high-performance distributed computing framework"
)]
struct Args {
    /// Type of node to run
    #[arg(short, long, value_enum)]
    node_type: NodeType,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8000)]
    port: u16,

    /// Head node address (for worker nodes)
    #[arg(short, long, default_value = "127.0.0.1:8000")]
    head_addr: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

/// Node type for distributed computing
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum NodeType {
    Head,
    Worker,
}

/// Initialize logging with dynamic log level
fn setup_logging(level: &str) -> Result<()> {
    let log_level = match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => {
            warn!("Invalid log level '{}', defaulting to INFO", level);
            Level::INFO
        }
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow::anyhow!("Failed to set logging subscriber: {}", e))?;

    Ok(())
}

/// Graceful shutdown handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler")
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install terminate signal handler")
            .recv()
            .await
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, initiating shutdown"),
        _ = terminate => info!("Received termination signal, initiating shutdown"),
    }
}

/// Main application entry point
#[tokio::main]
async fn main() -> Result<()> {
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    let args = Args::parse();
    let port = args.port;
    let grpc_addr = format!("0.0.0.0:{}", port).parse().unwrap();

    // 创建关闭通道
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
    
    // 监听 Ctrl+C 信号
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Received Ctrl+C, initiating shutdown...");
                shutdown_tx.send(()).await.ok();
            }
            Err(err) => {
                error!("Failed to listen for Ctrl+C: {}", err);
            }
        }
    });

    match args.node_type {
        NodeType::Head => {
            info!("Starting head node on port {}", port);
            
            let config = HeadNodeConfig {
                address: "0.0.0.0".to_string(),
                port,
                heartbeat_timeout: Duration::from_secs(30),
                resource_monitor_interval: Duration::from_secs(10),
                scheduling_strategy: LoadBalanceStrategy::RoundRobin,
            };

            let object_store = Arc::new(ObjectStore::new("default".to_string()));
            let metrics = Arc::new(MetricsCollector::new());
            let (status_tx, _) = mpsc::channel(100);

            let head = HeadNode::new(config, object_store, metrics.clone(), status_tx);
            head.start().await?;

            let worker = Arc::new(WorkerNode::new(
                "0.0.0.0".to_string(),
                port,
                metrics.clone(),
            ));

            let service = RustRayService::new(head, worker);
            
            info!("gRPC server listening on {}", grpc_addr);
            Server::builder()
                .add_service(RustrayServer::new(service))
                .serve_with_shutdown(grpc_addr, async {
                    shutdown_rx.recv().await;
                })
                .await?;
        }
        NodeType::Worker => {
            info!("Starting worker node on port {}", port);
            
            let metrics = Arc::new(MetricsCollector::new());
            let worker = Arc::new(WorkerNode::new(
                "0.0.0.0".to_string(),
                port,
                metrics.clone(),
            ));

            let service = RustRayService::new(HeadNode::default(), worker);
            
            info!("gRPC server listening on {}", grpc_addr);
            Server::builder()
                .add_service(RustrayServer::new(service))
                .serve_with_shutdown(grpc_addr, async {
                    shutdown_rx.recv().await;
                })
                .await?;
        }
    }

    info!("Server shutdown complete");
    Ok(())
} 