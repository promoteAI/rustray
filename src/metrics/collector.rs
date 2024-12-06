use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct TaskExecution {
    pub task_id: Uuid,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub cpu_usage: f64,
    pub memory_usage: u64,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub disk_usage: u64,
    pub network_bandwidth: f64,
}

#[derive(Debug, Clone)]
pub enum MetricType {
    Counter(f64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, MetricType>>>,
    task_executions: Arc<RwLock<HashMap<Uuid, TaskExecution>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            task_executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_metric(&self, name: &str, value: MetricType) {
        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), value);
    }

    pub async fn get_metric(&self, name: &str) -> Option<MetricType> {
        let metrics = self.metrics.read().await;
        metrics.get(name).cloned()
    }

    pub async fn record_task_execution(&self, execution: TaskExecution) {
        let mut executions = self.task_executions.write().await;
        executions.insert(execution.task_id, execution);
    }

    pub async fn get_task_execution(&self, task_id: Uuid) -> Option<TaskExecution> {
        let executions = self.task_executions.read().await;
        executions.get(&task_id).cloned()
    }
} 