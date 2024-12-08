use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::error::AppError;
use crate::models::Settings;
use crate::state::AppState;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/settings")
            .route("", web::get().to(get_settings))
            .route("", web::post().to(update_settings))
            .route("/reset", web::post().to(reset_settings))
    );
}

async fn get_settings(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let settings = state.settings.read().await;
    Ok(HttpResponse::Ok().json(&*settings))
}

async fn update_settings(
    state: web::Data<AppState>,
    settings: web::Json<Settings>,
) -> Result<HttpResponse, AppError> {
    let mut current_settings = state.settings.write().await;
    *current_settings = settings.into_inner();
    Ok(HttpResponse::Ok().json(&*current_settings))
}

async fn reset_settings(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let mut current_settings = state.settings.write().await;
    *current_settings = Settings::default();
    Ok(HttpResponse::Ok().json(&*current_settings))
} 