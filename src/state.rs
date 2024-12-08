use std::sync::Arc;
use tokio::sync::RwLock;
use sqlx::SqlitePool;

use crate::models::Settings;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub settings: Arc<RwLock<Settings>>,
}

impl AppState {
    pub async fn new() -> Self {
        // 创建SQLite数据库连接池
        let db = SqlitePool::connect("sqlite:data.db")
            .await
            .expect("Failed to connect to database");
            
        // 初始化数据库
        sqlx::migrate!("./src/migrations")
            .run(&db)
            .await
            .expect("Failed to run database migrations");
            
        // 加载设置
        let settings = Arc::new(RwLock::new(Settings::default()));
        
        Self { db, settings }
    }
} 