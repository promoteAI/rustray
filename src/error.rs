use actix_web::{error::ResponseError, HttpResponse};
use thiserror::Error;
use serde_json::json;

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