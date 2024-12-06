pub mod data_aware_scheduler;
pub mod load_balancer;
pub mod task_graph;

use crate::common::{NodeInfo, TaskSpec, TaskStatus};
use crate::metrics::collector::MetricsCollector;
use std::sync::Arc;

/// Node health status
#[derive(Debug, Clone, PartialEq)]
pub enum NodeHealth {
    Healthy,
    Unhealthy(String),
}

/// Scheduling strategy
#[derive(Debug, Clone)]
pub enum LoadBalanceStrategy {
    /// Round robin scheduling
    RoundRobin,
    /// Least loaded node first
    LeastLoaded,
    /// Data locality aware scheduling
    DataAware,
    /// Custom scheduling strategy
    Custom(String),
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        Self::LeastLoaded
    }
} 