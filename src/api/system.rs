use actix_web::{web, HttpResponse};
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::error::AppError;
use crate::system::SystemMonitor;

static SYSTEM_MONITOR: Lazy<Mutex<SystemMonitor>> = Lazy::new(|| {
    Mutex::new(SystemMonitor::new())
});

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/system")
            .route("/status", web::get().to(get_system_status))
    );
}

async fn get_system_status() -> Result<HttpResponse, AppError> {
    let mut monitor = SYSTEM_MONITOR.lock()
        .map_err(|e| AppError::System(format!("Failed to lock system monitor: {}", e)))?;
        
    let status = monitor.get_status()?;
    Ok(HttpResponse::Ok().json(status))
}