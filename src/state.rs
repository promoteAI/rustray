use std::sync::Arc;
use tokio::sync::RwLock;
use crate::metrics::collector::MetricsCollector;
use crate::worker::WorkerNode;

#[derive(Clone)]
pub struct AppState {
    pub metrics: Arc<RwLock<MetricsCollector>>,
    pub worker: Arc<RwLock<WorkerNode>>,
}

impl AppState {
    pub fn new(metrics: MetricsCollector, worker: WorkerNode) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(metrics)),
            worker: Arc::new(RwLock::new(worker)),
        }
    }
} 