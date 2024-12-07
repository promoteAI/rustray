pub type Result<T> = std::result::Result<T, RustRayError>;

#[derive(Debug, thiserror::Error)]
pub enum RustRayError {
    #[error("Internal error: {0}")]
    InternalError(String),
    
    #[error("Task error: {0}")]
    TaskError(String),
    
    #[error("Communication error: {0}")]
    CommunicationError(String),
    
    #[error("Resource error: {0}")]
    ResourceError(String),
}

impl From<tonic::transport::Error> for RustRayError {
    fn from(err: tonic::transport::Error) -> Self {
        RustRayError::CommunicationError(err.to_string())
    }
}

impl From<anyhow::Error> for RustRayError {
    fn from(err: anyhow::Error) -> Self {
        RustRayError::InternalError(err.to_string())
    }
}

impl From<String> for RustRayError {
    fn from(error: String) -> Self {
        RustRayError::InternalError(error)
    }
}

impl From<axum::Error> for RustRayError {
    fn from(err: axum::Error) -> Self {
        RustRayError::CommunicationError(err.to_string())
    }
}

impl From<hyper::Error> for RustRayError {
    fn from(err: hyper::Error) -> Self {
        RustRayError::CommunicationError(err.to_string())
    }
} 