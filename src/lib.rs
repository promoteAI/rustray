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