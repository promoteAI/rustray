pub mod api;
pub mod common;
pub mod error;
pub mod grpc;
pub mod head;
pub mod metrics;
pub mod scheduler;
pub mod worker;

pub mod proto {
    tonic::include_proto!("rustray");
}

pub use error::{Result, RustRayError};
pub use metrics::MetricsCollector;
pub use proto::*;

use std::sync::Arc;
use crate::worker::WorkerNode;

#[derive(Clone)]
pub struct AppState {
    pub metrics: Arc<MetricsCollector>,
    pub worker: Arc<WorkerNode>,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}