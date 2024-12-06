use std::fmt;
use std::error::Error;

pub type Result<T> = std::result::Result<T, RustRayError>;

#[derive(Debug)]
pub enum RustRayError {
    InternalError(String),
    TaskError(String),
    NetworkError(String),
    StorageError(String),
    CommunicationError(String),
    ResourceError(String),
    ConfigError(String),
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
        }
    }
}

impl Error for RustRayError {} 