use std::fmt;
use thiserror::Error;
use tonic::Status;

#[derive(Error, Debug)]
pub enum RustRayError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Task error: {0}")]
    TaskError(String),

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("Scheduler error: {0}")]
    SchedulerError(String),

    #[error("Object store error: {0}")]
    ObjectStoreError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("Resource error: {0}")]
    ResourceError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Dependency error: {0}")]
    DependencyError(String),

    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

impl From<RustRayError> for Status {
    fn from(error: RustRayError) -> Self {
        match error {
            RustRayError::IoError(e) => Status::internal(format!("IO error: {}", e)),
            RustRayError::TaskError(msg) => Status::failed_precondition(msg),
            RustRayError::WorkerError(msg) => Status::unavailable(msg),
            RustRayError::SchedulerError(msg) => Status::internal(msg),
            RustRayError::ObjectStoreError(msg) => Status::internal(msg),
            RustRayError::AuthenticationError(msg) => Status::unauthenticated(msg),
            RustRayError::AuthorizationError(msg) => Status::permission_denied(msg),
            RustRayError::ConfigurationError(msg) => Status::failed_precondition(msg),
            RustRayError::CommunicationError(msg) => Status::unavailable(msg),
            RustRayError::ResourceError(msg) => Status::resource_exhausted(msg),
            RustRayError::TimeoutError(msg) => Status::deadline_exceeded(msg),
            RustRayError::ValidationError(msg) => Status::invalid_argument(msg),
            RustRayError::InternalError(msg) => Status::internal(msg),
            RustRayError::NotFound(msg) => Status::not_found(msg),
            RustRayError::AlreadyExists(msg) => Status::already_exists(msg),
            RustRayError::InvalidArgument(msg) => Status::invalid_argument(msg),
            RustRayError::SerializationError(msg) => Status::internal(msg),
            RustRayError::DatabaseError(msg) => Status::internal(msg),
            RustRayError::NetworkError(msg) => Status::unavailable(msg),
            RustRayError::DependencyError(msg) => Status::failed_precondition(msg),
            RustRayError::TaskExecutionFailed(msg) => Status::failed_precondition(msg),
            RustRayError::AuthError(msg) => Status::unauthenticated(msg),
            RustRayError::ConfigError(msg) => Status::failed_precondition(msg),
        }
    }
}

impl From<Status> for RustRayError {
    fn from(status: Status) -> Self {
        match status.code() {
            tonic::Code::NotFound => RustRayError::NotFound(status.message().to_string()),
            tonic::Code::AlreadyExists => RustRayError::AlreadyExists(status.message().to_string()),
            tonic::Code::InvalidArgument => RustRayError::InvalidArgument(status.message().to_string()),
            tonic::Code::FailedPrecondition => RustRayError::ConfigurationError(status.message().to_string()),
            tonic::Code::Unauthenticated => RustRayError::AuthenticationError(status.message().to_string()),
            tonic::Code::PermissionDenied => RustRayError::AuthorizationError(status.message().to_string()),
            tonic::Code::ResourceExhausted => RustRayError::ResourceError(status.message().to_string()),
            tonic::Code::DeadlineExceeded => RustRayError::TimeoutError(status.message().to_string()),
            _ => RustRayError::InternalError(status.message().to_string()),
        }
    }
}

impl From<tokio::time::error::Elapsed> for RustRayError {
    fn from(error: tokio::time::error::Elapsed) -> Self {
        RustRayError::TimeoutError(error.to_string())
    }
}

impl From<serde_json::Error> for RustRayError {
    fn from(error: serde_json::Error) -> Self {
        RustRayError::SerializationError(error.to_string())
    }
}

impl From<std::str::Utf8Error> for RustRayError {
    fn from(error: std::str::Utf8Error) -> Self {
        RustRayError::SerializationError(error.to_string())
    }
}

impl From<std::string::FromUtf8Error> for RustRayError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        RustRayError::SerializationError(error.to_string())
    }
}

impl From<std::num::ParseIntError> for RustRayError {
    fn from(error: std::num::ParseIntError) -> Self {
        RustRayError::ValidationError(error.to_string())
    }
}

impl From<std::num::ParseFloatError> for RustRayError {
    fn from(error: std::num::ParseFloatError) -> Self {
        RustRayError::ValidationError(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, RustRayError>;

#[derive(Debug)]
pub struct ErrorContext {
    pub error: RustRayError,
    pub context: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub backtrace: Option<String>,
}

impl ErrorContext {
    pub fn new(error: RustRayError, context: String) -> Self {
        let backtrace = std::backtrace::Backtrace::capture();
        Self {
            error,
            context,
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            backtrace: Some(format!("{:?}", backtrace)),
        }
    }

    pub fn add_context(self, additional_context: String) -> Self {
        Self {
            context: format!("{}: {}", self.context, additional_context),
            ..self
        }
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Error: {}", self.error)?;
        writeln!(f, "Context: {}", self.context)?;
        writeln!(f, "Location: {}:{}:{}", self.file, self.line, self.column)?;
        if let Some(backtrace) = &self.backtrace {
            writeln!(f, "Backtrace:\n{}", backtrace)?;
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! with_context {
    ($result:expr, $context:expr) => {
        match $result {
            Ok(value) => Ok(value),
            Err(error) => {
                let context = $crate::error::ErrorContext::new(error.into(), $context.into());
                Err(context)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let error = RustRayError::NotFound("test".to_string());
        let status: Status = error.into();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[test]
    fn test_error_context() {
        let error = RustRayError::TaskError("task failed".to_string());
        let context = ErrorContext::new(error, "executing task".to_string());
        let context = context.add_context("retry attempt 1".to_string());
        
        assert!(context.to_string().contains("executing task"));
        assert!(context.to_string().contains("retry attempt 1"));
    }

    #[test]
    fn test_with_context_macro() {
        fn may_fail() -> Result<()> {
            Err(RustRayError::TaskError("task failed".to_string()))
        }

        let result = with_context!(may_fail(), "executing critical task");
        assert!(result.is_err());
    }
} 