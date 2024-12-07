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
    proto::rust_ray_server::RustRayServer,
    grpc::RustRayService,
    head::{HeadNode, HeadNodeConfig},
    worker::WorkerNode,
    error::Result,
    common::object_store::ObjectStore,
    metrics::MetricsCollector,
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
        .route("/api/system/overview", get(api::system::get_system_overview))
        .route("/api/system/metrics", get(api::system::get_system_metrics))
        .route("/api/metrics/cpu", get(api::system::get_cpu_metrics))
        .route("/api/metrics/memory", get(api::system::get_memory_metrics))
        .route("/api/metrics/network", get(api::system::get_network_metrics))
        .route("/api/metrics/storage", get(api::system::get_storage_metrics))
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
    // Parse command-line arguments
    let args = Args::parse();

    // Setup logging
    setup_logging(&args.log_level)?;

    // Configure server address
    let addr: SocketAddr = format!("0.0.0.0:{}", args.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

    // Spawn shutdown signal handler
    let shutdown_handle = tokio::spawn(shutdown_signal());

    match args.node_type {
        NodeType::Head => {
            info!("Starting head node on {}", addr);
            
            // Create head node configuration
            let config = HeadNodeConfig {
                address: addr.ip().to_string(),
                port: addr.port(),
                heartbeat_timeout: Duration::from_secs(30),
                resource_monitor_interval: Duration::from_secs(10),
                scheduling_strategy: rustray::scheduler::LoadBalanceStrategy::RoundRobin,
            };

            // Create shared components
            let object_store = Arc::new(ObjectStore::new("default".to_string()));
            let metrics = Arc::new(MetricsCollector::new("head".to_string()));
            let (status_tx, _status_rx) = mpsc::channel(100);

            // Create head node
            let head = HeadNode::new(
                config,
                object_store,
                metrics.clone(),
                status_tx,
            );

            // Create worker node for local tasks
            let worker = WorkerNode::new(
                addr.ip().to_string(),
                addr.port(),
                metrics,
            );

            // Create gRPC service
            let service = RustRayService::new(head, worker);
            
            let server = Server::builder()
                .add_service(RustRayServer::new(service))
                .serve_with_shutdown(addr, async {
                    shutdown_handle.await.ok();
                });

            info!("Head node server started successfully");
            
            if let Err(e) = server.await {
                error!("Head node server error: {}", e);
                return Err(e.into());
            }
        }
        NodeType::Worker => {
            info!("Starting worker node on {} (head: {})", addr, args.head_addr);
            
            // Create metrics collector
            let metrics = Arc::new(MetricsCollector::new("worker".to_string()));

            // Create worker node
            let worker = WorkerNode::new(
                addr.ip().to_string(),
                addr.port(),
                metrics,
            );
            
            tokio::select! {
                result = worker.connect_to_head(&args.head_addr) => {
                    match result {
                        Ok(_) => info!("Successfully connected to head node"),
                        Err(e) => {
                            error!("Failed to connect to head node: {}", e);
                            return Err(e);
                        }
                    }
                }
                _ = shutdown_handle => {
                    info!("Shutdown signal received before connection");
                }
            }
        }
    }

    info!("Node shutdown complete");
    Ok(())
} 