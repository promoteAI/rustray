use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;
use std::time::Instant;
use anyhow::Result;

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

    pub async fn record_metric(&self, name: &str, value: MetricType) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), value);
        Ok(())
    }

    pub async fn get_metric(&self, name: &str) -> Result<Option<MetricType>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.get(name).cloned())
    }

    pub async fn increment_counter(&self, name: &str, value: u64) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        let counter = metrics.entry(name.to_string())
            .or_insert(MetricType::Counter(0.0));
        
        if let MetricType::Counter(ref mut count) = counter {
            *count += value as f64;
        }
        
        Ok(())
    }

    pub async fn set_gauge(&self, name: &str, value: f64) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), MetricType::Gauge(value));
        Ok(())
    }

    pub async fn record_histogram(&self, name: &str, value: f64) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        let histogram = metrics.entry(name.to_string())
            .or_insert(MetricType::Histogram(Vec::new()));
        
        if let MetricType::Histogram(ref mut values) = histogram {
            values.push(value);
        }
        
        Ok(())
    }

    pub async fn record_task_execution(&self, execution: TaskExecution) -> Result<()> {
        let mut executions = self.task_executions.write().await;
        executions.insert(execution.task_id, execution);
        Ok(())
    }

    pub async fn get_task_execution(&self, task_id: Uuid) -> Result<Option<TaskExecution>> {
        let executions = self.task_executions.read().await;
        Ok(executions.get(&task_id).cloned())
    }
} 