use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use crate::worker::SystemMetricsResponse;
use crate::error::{Result, RustRayError};

#[derive(Debug)]
pub struct MetricsCollector {
    counters: Mutex<HashMap<String, i64>>,
    gauges: Mutex<HashMap<String, f64>>,
    histograms: Mutex<HashMap<String, Vec<f64>>>,
    timers: Mutex<HashMap<String, Instant>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Mutex::new(HashMap::new()),
            gauges: Mutex::new(HashMap::new()),
            histograms: Mutex::new(HashMap::new()),
            timers: Mutex::new(HashMap::new()),
        }
    }

    pub fn increment_counter(&self, key: &str, value: i64) -> Result<()> {
        let mut counters = self.counters.lock()
            .map_err(|e| RustRayError::InternalError(format!("Failed to lock counters: {}", e)))?;
        *counters.entry(key.to_string()).or_insert(0) += value;
        Ok(())
    }

    pub fn set_gauge(&self, key: &str, value: f64) -> Result<()> {
        let mut gauges = self.gauges.lock()
            .map_err(|e| RustRayError::InternalError(format!("Failed to lock gauges: {}", e)))?;
        gauges.insert(key.to_string(), value);
        Ok(())
    }

    pub fn record_histogram(&self, key: &str, value: f64) -> Result<()> {
        let mut histograms = self.histograms.lock()
            .map_err(|e| RustRayError::InternalError(format!("Failed to lock histograms: {}", e)))?;
        histograms.entry(key.to_string()).or_insert_with(Vec::new).push(value);
        Ok(())
    }

    pub fn start_timer(&self, key: &str) -> Result<()> {
        let mut timers = self.timers.lock()
            .map_err(|e| RustRayError::InternalError(format!("Failed to lock timers: {}", e)))?;
        timers.insert(key.to_string(), Instant::now());
        Ok(())
    }

    pub fn stop_timer(&self, key: &str) -> Result<f64> {
        let mut timers = self.timers.lock()
            .map_err(|e| RustRayError::InternalError(format!("Failed to lock timers: {}", e)))?;
        if let Some(start_time) = timers.remove(key) {
            Ok(start_time.elapsed().as_secs_f64())
        } else {
            Err(RustRayError::InternalError(format!("Timer {} not found", key)))
        }
    }

    pub fn get_counter(&self, key: &str) -> Result<i64> {
        let counters = self.counters.lock()
            .map_err(|e| RustRayError::InternalError(format!("Failed to lock counters: {}", e)))?;
        Ok(*counters.get(key).unwrap_or(&0))
    }

    pub fn get_gauge(&self, key: &str) -> Result<f64> {
        let gauges = self.gauges.lock()
            .map_err(|e| RustRayError::InternalError(format!("Failed to lock gauges: {}", e)))?;
        Ok(*gauges.get(key).unwrap_or(&0.0))
    }

    pub fn get_histogram(&self, key: &str) -> Result<Vec<f64>> {
        let histograms = self.histograms.lock()
            .map_err(|e| RustRayError::InternalError(format!("Failed to lock histograms: {}", e)))?;
        Ok(histograms.get(key).cloned().unwrap_or_default())
    }

    pub fn record_system_metrics(&self, metrics: &SystemMetricsResponse) -> Result<()> {
        // Record CPU metrics
        for (i, usage) in metrics.cpu.usage.iter().enumerate() {
            self.set_gauge(&format!("system.cpu.core_{}.usage", i), *usage)?;
        }
        self.set_gauge("system.cpu.cores", metrics.cpu.cores as f64)?;

        // Record memory metrics
        self.set_gauge("system.memory.total", metrics.memory.total as f64)?;
        self.set_gauge("system.memory.used", metrics.memory.used as f64)?;
        self.set_gauge("system.memory.free", metrics.memory.free as f64)?;
        self.set_gauge("system.memory.usage_percentage", metrics.memory.usage_percentage)?;

        // Record network metrics
        self.set_gauge("system.network.bytes_sent", metrics.network.bytes_sent as f64)?;
        self.set_gauge("system.network.bytes_recv", metrics.network.bytes_recv as f64)?;
        self.set_gauge("system.network.packets_sent", metrics.network.packets_sent as f64)?;
        self.set_gauge("system.network.packets_recv", metrics.network.packets_recv as f64)?;

        // Record storage metrics
        self.set_gauge("system.storage.total", metrics.storage.total as f64)?;
        self.set_gauge("system.storage.used", metrics.storage.used as f64)?;
        self.set_gauge("system.storage.free", metrics.storage.free as f64)?;
        self.set_gauge("system.storage.usage_percentage", metrics.storage.usage_percentage)?;

        Ok(())
    }
}
