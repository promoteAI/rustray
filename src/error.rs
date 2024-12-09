use actix_web::{error::ResponseError, HttpResponse};
use thiserror::Error;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use log::{error, warn};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("System error: {0}")]
    System(String),
    
    #[error("Task error: {0}")]
    Task(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid request: {0}")]
    BadRequest(String),
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::Database(e) => {
                log::error!("Database error: {}", e);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Internal server error"
                }))
            }
            AppError::System(msg) => {
                log::error!("System error: {}", msg);
                HttpResponse::InternalServerError().json(json!({
                    "error": msg
                }))
            }
            AppError::Task(msg) => {
                log::error!("Task error: {}", msg);
                HttpResponse::BadRequest().json(json!({
                    "error": msg
                }))
            }
            AppError::NotFound(msg) => {
                HttpResponse::NotFound().json(json!({
                    "error": msg
                }))
            }
            AppError::BadRequest(msg) => {
                HttpResponse::BadRequest().json(json!({
                    "error": msg
                }))
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum SystemError {
    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Task execution error: {0}")]
    TaskError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Operation timeout: {0}")]
    Timeout(String),

    #[error("System overload: {0}")]
    Overload(String),
}

pub type SystemResult<T> = Result<T, SystemError>;

pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_factor: 2.0,
        }
    }
}

pub async fn retry_with_backoff<T, F, Fut>(
    operation: F,
    config: RetryConfig,
) -> SystemResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = SystemResult<T>>,
{
    let mut retries = 0;
    let mut delay = config.initial_delay;

    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if retries >= config.max_retries {
                    error!("Operation failed after {} retries: {:?}", retries, err);
                    return Err(err);
                }

                warn!("Operation failed (attempt {}): {:?}", retries + 1, err);
                sleep(delay).await;

                retries += 1;
                delay = std::cmp::min(
                    delay.mul_f64(config.backoff_factor),
                    config.max_delay,
                );
            }
        }
    }
}

pub struct CircuitBreaker {
    failure_threshold: u32,
    reset_timeout: Duration,
    failure_count: u32,
    last_failure: Option<std::time::Instant>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            failure_count: 0,
            last_failure: None,
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(std::time::Instant::now());
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.last_failure = None;
    }

    pub fn is_open(&self) -> bool {
        if self.failure_count >= self.failure_threshold {
            if let Some(last_failure) = self.last_failure {
                if last_failure.elapsed() < self.reset_timeout {
                    return true;
                }
            }
        }
        false
    }
}

pub async fn with_circuit_breaker<T, F, Fut>(
    breaker: &mut CircuitBreaker,
    operation: F,
) -> SystemResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = SystemResult<T>>,
{
    if breaker.is_open() {
        return Err(SystemError::Overload(
            "Circuit breaker is open".to_string(),
        ));
    }

    match operation().await {
        Ok(result) => {
            breaker.record_success();
            Ok(result)
        }
        Err(err) => {
            breaker.record_failure();
            Err(err)
        }
    }
} 