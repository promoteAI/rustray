use std::time::Instant;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use log::{info, warn, error};

#[derive(Debug, Clone)]
pub struct Metrics {
    pub request_count: u64,
    pub error_count: u64,
    pub latency_ms: f64,
}

#[derive(Debug)]
pub struct SpanContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_id: Option<String>,
    pub start_time: Instant,
}

pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, Metrics>>>,
    traces: Arc<RwLock<HashMap<String, Vec<SpanContext>>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            traces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_request(&self, service: &str, latency: f64, is_error: bool) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics.entry(service.to_string())
            .or_insert(Metrics {
                request_count: 0,
                error_count: 0,
                latency_ms: 0.0,
            });
        
        entry.request_count += 1;
        if is_error {
            entry.error_count += 1;
        }
        entry.latency_ms = (entry.latency_ms * (entry.request_count - 1) as f64 + latency) 
            / entry.request_count as f64;
    }

    pub async fn start_span(&self, trace_id: &str, parent_id: Option<String>) -> SpanContext {
        let span = SpanContext {
            trace_id: trace_id.to_string(),
            span_id: uuid::Uuid::new_v4().to_string(),
            parent_id,
            start_time: Instant::now(),
        };

        let mut traces = self.traces.write().await;
        traces.entry(trace_id.to_string())
            .or_insert_with(Vec::new)
            .push(span.clone());

        span
    }

    pub async fn end_span(&self, span: SpanContext) {
        let duration = span.start_time.elapsed();
        info!(
            "Trace: {}, Span: {}, Parent: {:?}, Duration: {:?}",
            span.trace_id, span.span_id, span.parent_id, duration
        );
    }

    pub async fn get_service_metrics(&self, service: &str) -> Option<Metrics> {
        self.metrics.read().await.get(service).cloned()
    }

    pub async fn get_trace(&self, trace_id: &str) -> Option<Vec<SpanContext>> {
        self.traces.read().await.get(trace_id).cloned()
    }
}

pub mod collector;

pub use collector::MetricsCollector; 