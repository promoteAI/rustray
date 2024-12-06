use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::common::{NodeInfo, TaskSpec, TaskRequiredResources};
use crate::error::{Result, RustRayError};
use super::{LoadBalanceStrategy, NodeHealth};

pub struct LoadBalancer {
    strategy: LoadBalanceStrategy,
    nodes: Arc<RwLock<HashMap<Uuid, NodeInfo>>>,
    node_health: Arc<RwLock<HashMap<Uuid, NodeHealth>>>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        Self {
            strategy,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            node_health: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_node(&self, node: NodeInfo) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let mut health = self.node_health.write().await;
        
        let node_id = node.node_id;
        nodes.insert(node_id, node);
        health.insert(node_id, NodeHealth::Healthy);
        
        Ok(())
    }

    pub async fn select_node(&self, task: &TaskSpec) -> Option<Uuid> {
        let nodes = self.nodes.read().await;
        let health = self.node_health.read().await;
        
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

    pub async fn update_node_health(&self, node_id: Uuid, health: NodeHealth) -> Result<()> {
        let mut node_health = self.node_health.write().await;
        node_health.insert(node_id, health);
        Ok(())
    }
} 