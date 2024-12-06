use std::fmt;
use std::error::Error;

pub type Result<T, E = RustRayError> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum RustRayError {
    InternalError(String),
    TaskError(String),
    NetworkError(String),
    StorageError(String),
    CommunicationError(String),
    ResourceError(String),
    ConfigError(String),
    Other(String),
    TransportError(String),
}

impl fmt::Display for RustRayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RustRayError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            RustRayError::TaskError(msg) => write!(f, "Task error: {}", msg),
            RustRayError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            RustRayError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            RustRayError::CommunicationError(msg) => write!(f, "Communication error: {}", msg),
            RustRayError::ResourceError(msg) => write!(f, "Resource error: {}", msg),
            RustRayError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            RustRayError::Other(msg) => write!(f, "Other error: {}", msg),
            RustRayError::TransportError(msg) => write!(f, "Transport error: {}", msg),
        }
    }
}

impl Error for RustRayError {}

impl From<anyhow::Error> for RustRayError {
    fn from(err: anyhow::Error) -> Self {
        RustRayError::Other(err.to_string())
    }
}

impl From<tonic::transport::Error> for RustRayError {
    fn from(err: tonic::transport::Error) -> Self {
        RustRayError::TransportError(err.to_string())
    }
} 