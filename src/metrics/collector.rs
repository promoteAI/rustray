use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use metrics::{Counter, Gauge, Histogram};

#[derive(Debug, Clone)]
pub struct TaskExecution {
    pub task_id: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub worker_id: String,
    pub function_name: String,
    pub input_sizes: Vec<usize>,
    pub output_size: Option<usize>,
    pub memory_used: usize,
    pub cpu_usage: f64,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_usage: f64,
    pub memory_used: usize,
    pub memory_available: usize,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub disk_used: u64,
    pub disk_available: u64,
}

#[derive(Debug)]
pub enum MetricType {
    Counter(String),
    Gauge(String),
    Histogram(String),
}

pub struct MetricsCollector {
    // 任务执行历史
    task_history: Arc<RwLock<HashMap<String, TaskExecution>>>,
    
    // 资源使用情况
    resource_history: Arc<RwLock<Vec<(Instant, ResourceUsage)>>>,
    
    // 性能指标
    counters: HashMap<String, Counter>,
    gauges: HashMap<String, Gauge>,
    histograms: HashMap<String, Histogram>,
    
    // 配置
    history_window: Duration,
    sampling_interval: Duration,
}

impl MetricsCollector {
    pub fn new(history_window: Duration, sampling_interval: Duration) -> Self {
        // 初始化基本指标
        let mut counters = HashMap::new();
        let mut gauges = HashMap::new();
        let mut histograms = HashMap::new();

        // 任务相关指标
        counters.insert(
            "tasks_submitted".to_string(),
            Counter::new("tasks_submitted"),
        );
        counters.insert(
            "tasks_completed".to_string(),
            Counter::new("tasks_completed"),
        );
        counters.insert(
            "tasks_failed".to_string(),
            Counter::new("tasks_failed"),
        );

        // 资源相关指标
        gauges.insert(
            "cpu_usage".to_string(),
            Gauge::new("cpu_usage"),
        );
        gauges.insert(
            "memory_used".to_string(),
            Gauge::new("memory_used"),
        );
        gauges.insert(
            "memory_available".to_string(),
            Gauge::new("memory_available"),
        );

        // 延迟相关指标
        histograms.insert(
            "task_duration".to_string(),
            Histogram::new("task_duration"),
        );
        histograms.insert(
            "scheduling_latency".to_string(),
            Histogram::new("scheduling_latency"),
        );

        Self {
            task_history: Arc::new(RwLock::new(HashMap::new())),
            resource_history: Arc::new(RwLock::new(Vec::new())),
            counters,
            gauges,
            histograms,
            history_window,
            sampling_interval,
        }
    }

    pub async fn record_task_start(
        &self,
        task_id: String,
        worker_id: String,
        function_name: String,
        input_sizes: Vec<usize>,
    ) {
        let execution = TaskExecution {
            task_id: task_id.clone(),
            start_time: Instant::now(),
            end_time: None,
            worker_id,
            function_name,
            input_sizes,
            output_size: None,
            memory_used: 0,
            cpu_usage: 0.0,
        };

        self.task_history.write().await.insert(task_id, execution);
        self.counters.get("tasks_submitted")
            .unwrap()
            .increment(1);
    }

    pub async fn record_task_completion(
        &self,
        task_id: String,
        output_size: usize,
        memory_used: usize,
        cpu_usage: f64,
    ) {
        if let Some(mut execution) = self.task_history.write().await.get_mut(&task_id) {
            execution.end_time = Some(Instant::now());
            execution.output_size = Some(output_size);
            execution.memory_used = memory_used;
            execution.cpu_usage = cpu_usage;

            // 记录任务执行时间
            if let Some(duration) = execution.end_time.unwrap().checked_duration_since(execution.start_time) {
                self.histograms.get("task_duration")
                    .unwrap()
                    .record(duration.as_secs_f64());
            }
        }

        self.counters.get("tasks_completed")
            .unwrap()
            .increment(1);
    }

    pub async fn record_task_failure(&self, task_id: String, error: String) {
        if let Some(mut execution) = self.task_history.write().await.get_mut(&task_id) {
            execution.end_time = Some(Instant::now());
        }

        self.counters.get("tasks_failed")
            .unwrap()
            .increment(1);
    }

    pub async fn record_resource_usage(&self, usage: ResourceUsage) {
        let now = Instant::now();
        let mut history = self.resource_history.write().await;
        
        // 更新资源使用指标
        self.gauges.get("cpu_usage")
            .unwrap()
            .set(usage.cpu_usage);
        self.gauges.get("memory_used")
            .unwrap()
            .set(usage.memory_used as f64);
        self.gauges.get("memory_available")
            .unwrap()
            .set(usage.memory_available as f64);

        // 添加到历史记录
        history.push((now, usage));

        // 清理过期数据
        let cutoff = now - self.history_window;
        history.retain(|(t, _)| *t >= cutoff);
    }

    pub async fn get_task_stats(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        let history = self.task_history.read().await;

        let completed_tasks: Vec<_> = history.values()
            .filter(|task| task.end_time.is_some())
            .collect();

        if !completed_tasks.is_empty() {
            // 平均执行时间
            let avg_duration: f64 = completed_tasks.iter()
                .filter_map(|task| {
                    task.end_time?
                        .checked_duration_since(task.start_time)
                        .map(|d| d.as_secs_f64())
                })
                .sum::<f64>() / completed_tasks.len() as f64;
            stats.insert("avg_duration".to_string(), avg_duration);

            // 平均内存使用
            let avg_memory: f64 = completed_tasks.iter()
                .map(|task| task.memory_used as f64)
                .sum::<f64>() / completed_tasks.len() as f64;
            stats.insert("avg_memory".to_string(), avg_memory);

            // 平均CPU使用率
            let avg_cpu: f64 = completed_tasks.iter()
                .map(|task| task.cpu_usage)
                .sum::<f64>() / completed_tasks.len() as f64;
            stats.insert("avg_cpu".to_string(), avg_cpu);
        }

        stats
    }

    pub async fn get_resource_stats(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        let history = self.resource_history.read().await;

        if !history.is_empty() {
            // 平均CPU使用率
            let avg_cpu: f64 = history.iter()
                .map(|(_, usage)| usage.cpu_usage)
                .sum::<f64>() / history.len() as f64;
            stats.insert("avg_cpu_usage".to_string(), avg_cpu);

            // 平均内存使用
            let avg_memory: f64 = history.iter()
                .map(|(_, usage)| usage.memory_used as f64)
                .sum::<f64>() / history.len() as f64;
            stats.insert("avg_memory_used".to_string(), avg_memory);

            // 网络使用趋势
            let network_rx: f64 = history.iter()
                .map(|(_, usage)| usage.network_rx_bytes as f64)
                .sum::<f64>() / history.len() as f64;
            stats.insert("avg_network_rx".to_string(), network_rx);

            let network_tx: f64 = history.iter()
                .map(|(_, usage)| usage.network_tx_bytes as f64)
                .sum::<f64>() / history.len() as f64;
            stats.insert("avg_network_tx".to_string(), network_tx);
        }

        stats
    }
} 