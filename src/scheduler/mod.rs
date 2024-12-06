pub mod data_aware_scheduler;
pub mod load_balancer;
pub mod task_graph;

pub use self::load_balancer::LoadBalancer;
pub use self::task_graph::TaskGraph;

/// Node health status
#[derive(Debug, Clone, PartialEq)]
pub enum NodeHealth {
    Healthy,
    Unhealthy(String),
}

/// Load balancing strategy
#[derive(Debug, Clone)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastLoaded,
    DataAware,
    Custom(String),
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        Self::LeastLoaded
    }
}
  