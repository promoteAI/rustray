use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;
use crate::error::Result;
use tracing::warn;
use crate::worker::{SystemMetricsResponse};

/// Metrics collector for system monitoring
pub struct MetricsCollector {
    name: String,
    counters: RwLock<HashMap<String, i64>>,
    gauges: RwLock<HashMap<String, f64>>,
    histograms: RwLock<HashMap<String, Vec<f64>>>,
    timers: RwLock<HashMap<String, Instant>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(name: String) -> Self {
        Self {
            name,
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            timers: RwLock::new(HashMap::new()),
        }
    }

    /// Increment a counter by the given value
    pub async fn increment_counter(&self, key: &str, value: i64) -> Result<(), String> {
        let mut counters = self.counters.write().unwrap();
        let counter = counters.entry(key.to_string()).or_insert(0);
        *counter += value;
        Ok(())
    }

    /// Set a gauge value
    pub async fn set_gauge(&self, key: &str, value: f64) -> Result<(), String> {
        let mut gauges = self.gauges.write().unwrap();
        gauges.insert(key.to_string(), value);
        Ok(())
    }

    /// Record a value in a histogram
    pub async fn record_histogram(&self, key: &str, value: f64) -> Result<(), String> {
        let mut histograms = self.histograms.write().unwrap();
        let histogram = histograms.entry(key.to_string()).or_insert_with(Vec::new);
        histogram.push(value);
        Ok(())
    }

    /// Start a timer
    pub async fn start_timer(&self, key: &str) -> Result<(), String> {
        let mut timers = self.timers.write().unwrap();
        timers.insert(key.to_string(), Instant::now());
        Ok(())
    }

    /// Stop a timer and record the duration
    pub async fn stop_timer(&self, key: &str) -> Result<f64, String> {
        let mut timers = self.timers.write().unwrap();
        if let Some(start_time) = timers.remove(key) {
            let duration = start_time.elapsed();
            let duration_secs = duration.as_secs_f64();
            self.record_histogram(key, duration_secs).await?;
            Ok(duration_secs)
        } else {
            Err("Timer not found".to_string())
        }
    }

    /// Get a counter value
    pub fn get_counter(&self, key: &str) -> Option<i64> {
        let counters = self.counters.read().unwrap();
        counters.get(key).copied()
    }

    /// Get a gauge value
    pub fn get_gauge(&self, key: &str) -> Option<f64> {
        let gauges = self.gauges.read().unwrap();
        gauges.get(key).copied()
    }

    /// Get histogram values
    pub fn get_histogram(&self, key: &str) -> Option<Vec<f64>> {
        let histograms = self.histograms.read().unwrap();
        histograms.get(key).cloned()
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.counters.write().unwrap().clear();
        self.gauges.write().unwrap().clear();
        self.histograms.write().unwrap().clear();
        self.timers.write().unwrap().clear();
    }

    pub async fn record_system_metrics(&self, metrics: &SystemMetricsResponse) -> Result<(), String> {
        // 记录系统级别指标
        self.set_gauge("system.cpu.usage", metrics.cpu.usage.iter().sum::<f64>() / metrics.cpu.usage.len() as f64).await?;
        
        self.set_gauge("system.memory.usage_percentage", metrics.memory.usage_percentage).await?;
        
        self.set_gauge("system.network.bytes_sent", metrics.network.bytes_sent as f64).await?;
        
        Ok(())
    }
} 