use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub description: Option<String>,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub config: TaskConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskConfig {
    pub max_workers: u32,
    pub timeout: u32,
    pub retry_count: u32,
    pub use_gpu: bool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "UPPERCASE")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cpu: CpuStatus,
    pub memory: MemoryStatus,
    pub disk: DiskStatus,
    pub network: NetworkStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuStatus {
    pub usage: f32,
    pub cores: u32,
    pub load: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryStatus {
    pub usage: f32,
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskStatus {
    pub usage: f32,
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub upload_speed: u64,
    pub download_speed: u64,
    pub total_traffic: u64,
    pub connections: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    // 系统配置
    pub max_workers: u32,
    pub task_timeout: u32,
    pub log_level: String,
    
    // 性能设置
    pub max_memory: u32,
    pub cpu_priority: String,
    pub enable_gpu: bool,
    
    // 网络设置
    pub api_port: u16,
    pub max_connections: u32,
    pub enable_ssl: bool,
    
    // 存储设置
    pub data_dir: String,
    pub backup_interval: u32,
    pub enable_compression: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_workers: 4,
            task_timeout: 30,
            log_level: "info".to_string(),
            max_memory: 8,
            cpu_priority: "normal".to_string(),
            enable_gpu: true,
            api_port: 3000,
            max_connections: 100,
            enable_ssl: false,
            data_dir: "/data".to_string(),
            backup_interval: 24,
            enable_compression: true,
        }
    }
} 