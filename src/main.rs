use actix_cors::Cors;
use actix_web::{middleware::Logger, App, HttpServer};
use log::info;

mod api;
mod error;
mod models;
mod state;
mod system;

use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // 创建应用状态
    let state = AppState::new().await;
    
    info!("Starting server at http://localhost:3000");
    
    // 启动HTTP服务器
    HttpServer::new(move || {
        // 配置CORS
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
            
        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(state.clone())
            // API路由
            .configure(api::tasks::config)
            .configure(api::system::config)
            .configure(api::settings::config)
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
} 