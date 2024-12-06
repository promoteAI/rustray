pub mod common;
pub mod error;
pub mod scheduler;
pub mod metrics;
pub mod head;
pub mod worker;

// 导出生成的gRPC代码
pub mod proto {
    tonic::include_proto!("rustray");
}

// 导出gRPC服务实现
pub mod grpc;

pub use error::{Result, RustRayError};
pub use metrics::MetricsCollector;
pub use proto::*;