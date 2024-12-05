use thiserror::Error;
use tonic::Status;
use uuid::Error as UuidError;
use std::io;

/// RustRay 分布式系统中的错误类型
#[derive(Error, Debug)]
pub enum RustRayError {
    /// IO操作错误
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// UUID生成或解析错误
    #[error("UUID error: {0}")]
    Uuid(#[from] UuidError),

    /// gRPC通信错误
    #[error("gRPC error: {0}")]
    Grpc(#[from] Status),

    /// 找不到指定的工作节点
    #[error("Worker not found: {0}")]
    WorkerNotFound(String),

    /// 任务执行失败
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),

    /// 工作节点注册失败
    #[error("Worker registration failed: {0}")]
    WorkerRegistrationFailed(String),

    /// 配置无效
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// 节点间通信错误
    #[error("Communication error: {0}")]
    CommunicationError(String),

    /// 资源不可用
    #[error("Resource not available: {0}")]
    ResourceNotAvailable(String),

    /// 认证错误
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// 授权错误
    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    /// 任务调度错误
    #[error("Task scheduling error: {0}")]
    SchedulingError(String),

    /// 系统状态错误
    #[error("System state error: {0}")]
    SystemStateError(String),
}

/// 将RustRay错误转换为gRPC状态
impl From<RustRayError> for Status {
    fn from(error: RustRayError) -> Self {
        match error {
            RustRayError::WorkerNotFound(msg) => Status::not_found(msg),
            RustRayError::TaskExecutionFailed(msg) => Status::internal(msg),
            RustRayError::WorkerRegistrationFailed(msg) => Status::invalid_argument(msg),
            RustRayError::InvalidConfig(msg) => Status::failed_precondition(msg),
            RustRayError::CommunicationError(msg) => Status::unavailable(msg),
            RustRayError::ResourceNotAvailable(msg) => Status::resource_exhausted(msg),
            RustRayError::AuthenticationError(msg) => Status::unauthenticated(msg),
            RustRayError::AuthorizationError(msg) => Status::permission_denied(msg),
            RustRayError::SchedulingError(msg) => Status::aborted(msg),
            RustRayError::SystemStateError(msg) => Status::failed_precondition(msg),
            _ => Status::internal(error.to_string()),
        }
    }
}

/// RustRay系统的Result类型别名
pub type Result<T> = std::result::Result<T, RustRayError>; 