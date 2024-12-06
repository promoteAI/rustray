use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use crate::common::{NodeInfo, TaskSpec};
use crate::metrics::collector::MetricsCollector;
use super::{LoadBalanceStrategy, NodeHealth};

pub struct LoadBalancer {
    strategy: LoadBalanceStrategy,
    nodes: Arc<RwLock<HashMap<Uuid, NodeInfo>>>,
    node_health: Arc<RwLock<HashMap<Uuid, NodeHealth>>>,
    metrics: Arc<MetricsCollector>,
}

impl LoadBalancer {
    pub fn new(
        strategy: LoadBalanceStrategy,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            strategy,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            node_health: Arc::new(RwLock::new(HashMap::new())),
            metrics,
        }
    }

    pub fn register_node(&self, node: NodeInfo) -> anyhow::Result<()> {
        let mut nodes = self.nodes.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        let mut health = self.node_health.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        nodes.insert(node.node_id, node);
        health.insert(node.node_id, NodeHealth::Healthy);
        
        Ok(())
    }

    pub fn select_node(&self, task: &TaskSpec) -> Option<Uuid> {
        let nodes = self.nodes.read().ok()?;
        let health = self.node_health.read().ok()?;
        
        let healthy_nodes: Vec<_> = nodes.iter()
            .filter(|(id, _)| matches!(health.get(id), Some(NodeHealth::Healthy)))
            .collect();
            
        if healthy_nodes.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                // Simple round-robin selection
                Some(*healthy_nodes[0].0)
            }
            LoadBalanceStrategy::LeastLoaded => {
                // TODO: Implement least loaded selection
                Some(*healthy_nodes[0].0)
            }
            LoadBalanceStrategy::DataAware => {
                // TODO: Implement data-aware selection
                Some(*healthy_nodes[0].0)
            }
            LoadBalanceStrategy::Custom(_) => {
                // TODO: Implement custom selection
                Some(*healthy_nodes[0].0)
            }
        }
    }

    pub fn update_node_health(&self, node_id: Uuid, health: NodeHealth) -> anyhow::Result<()> {
        let mut node_health = self.node_health.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        node_health.insert(node_id, health);
        Ok(())
    }
} 