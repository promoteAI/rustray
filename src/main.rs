//! RustRay Distributed Task Processing Framework

use std::net::SocketAddr;
use std::sync::Arc;
use axum::Router;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use rustray::{
    api::system,
    metrics::MetricsCollector,
    worker::WorkerNode,
    AppState,
};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create MetricsCollector
    let metrics = MetricsCollector::new();
    let metrics_arc = Arc::new(metrics.clone());
    
    // Create WorkerNode
    let worker = WorkerNode::new(
        "127.0.0.1".to_string(),
        50051,
        metrics_arc,
    );

    // Create application state
    let state = AppState::new(metrics, worker);

    // Create router
    let app = Router::new()
        .merge(system::router())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, starting graceful shutdown");
} 