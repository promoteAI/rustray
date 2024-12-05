use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use metrics::{Counter, Gauge, Histogram};

/// 性能指标类型
#[derive(Debug, Clone)]
pub enum MetricType {
    Counter(String),
    Gauge(String),
    Histogram(String),
}

/// 性能指标收集器
pub struct MetricsCollector {
    // 计数器
    counters: HashMap<String, Counter>,
    // 仪表盘
    gauges: HashMap<String, Gauge>,
    // 直方图
    histograms: HashMap<String, Histogram>,
    // 任务执行历史
    task_history: Arc<RwLock<HashMap<String, Vec<TaskExecution>>>>,
}

/// 任务执行记录
#[derive(Debug, Clone)]
pub struct TaskExecution {
    pub task_id: String,
    pub worker_id: String,
    pub start_time: Instant,
    pub duration: Duration,
    pub input_data_size: usize,
    pub output_data_size: usize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            task_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 记录任务执行
    pub async fn record_task_execution(&self, execution: TaskExecution) {
        let mut history = self.task_history.write().await;
        history.entry(execution.worker_id.clone())
            .or_default()
            .push(execution.clone());
        
        // 更新相关指标
        self.update_task_metrics(&execution).await;
    }

    /// 更新任务相关指标
    async fn update_task_metrics(&self, execution: &TaskExecution) {
        // 更新任务计数
        if let Some(counter) = self.counters.get("tasks_completed") {
            counter.increment(1);
        }
        
        // 更新执行时间直方图
        if let Some(histogram) = self.histograms.get("task_duration") {
            histogram.record(execution.duration.as_secs_f64());
        }
        
        // 更新数据传输量
        if let Some(counter) = self.counters.get("data_transferred") {
            counter.increment(execution.input_data_size as u64);
            counter.increment(execution.output_data_size as u64);
        }
    }

    /// 获取节点的平均任务执行时间
    pub async fn get_avg_execution_time(&self, worker_id: &str) -> Option<Duration> {
        let history = self.task_history.read().await;
        history.get(worker_id).map(|executions| {
            let total_duration: Duration = executions.iter()
                .map(|e| e.duration)
                .sum();
            total_duration / executions.len() as u32
        })
    }

    /// 获取节点的任务成功率
    pub async fn get_success_rate(&self, worker_id: &str) -> f64 {
        if let Some(counter) = self.counters.get(&format!("tasks_completed_{}", worker_id)) {
            let completed = counter.get();
            if let Some(total) = self.counters.get(&format!("tasks_total_{}", worker_id)) {
                return completed as f64 / total.get() as f64;
            }
        }
        0.0
    }

    /// 获取节点的资源利用率历史
    pub async fn get_resource_usage_history(&self, worker_id: &str) -> Vec<ResourceUsage> {
        let mut usage_history = Vec::new();
        if let Some(cpu) = self.histograms.get(&format!("cpu_usage_{}", worker_id)) {
            if let Some(memory) = self.histograms.get(&format!("memory_usage_{}", worker_id)) {
                // 获取最近的样本
                usage_history.push(ResourceUsage {
                    cpu: cpu.get_snapshot().value(),
                    memory: memory.get_snapshot().value(),
                    timestamp: Instant::now(),
                });
            }
        }
        usage_history
    }

    /// 导出指标数据
    pub async fn export_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        
        // 导出计数器
        for (name, counter) in &self.counters {
            metrics.insert(name.clone(), counter.get() as f64);
        }
        
        // 导出仪表盘
        for (name, gauge) in &self.gauges {
            metrics.insert(name.clone(), gauge.get());
        }
        
        // 导出直方图统计
        for (name, histogram) in &self.histograms {
            let snapshot = histogram.get_snapshot();
            metrics.insert(format!("{}_mean", name), snapshot.mean());
            metrics.insert(format!("{}_p95", name), snapshot.value_at_quantile(0.95));
        }
        
        metrics
    }
}

/// 资源使用记录
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu: f64,
    pub memory: f64,
    pub timestamp: Instant,
} 