use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::common::{NodeInfo, TaskSpec};
use crate::error::Result;
use crate::metrics::MetricsCollector;
use super::{LoadBalanceStrategy, NodeHealth};
use super::data_aware_scheduler::DataAwareScheduler;

pub struct LoadBalancer {
    strategy: LoadBalanceStrategy,
    nodes: Arc<RwLock<HashMap<Uuid, NodeInfo>>>,
    node_health: Arc<RwLock<HashMap<Uuid, NodeHealth>>>,
    node_tasks: Arc<RwLock<HashMap<Uuid, usize>>>,
    data_scheduler: Arc<DataAwareScheduler>,
    metrics: Arc<MetricsCollector>,
    current_round: Arc<RwLock<usize>>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalanceStrategy, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            strategy,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            node_health: Arc::new(RwLock::new(HashMap::new())),
            node_tasks: Arc::new(RwLock::new(HashMap::new())),
            data_scheduler: Arc::new(DataAwareScheduler::new(metrics.clone())),
            metrics,
            current_round: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn register_node(&self, node: NodeInfo) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let mut health = self.node_health.write().await;
        let mut tasks = self.node_tasks.write().await;
        
        let node_id = node.node_id;
        nodes.insert(node_id, node);
        health.insert(node_id, NodeHealth::Healthy);
        tasks.insert(node_id, 0);
        
        Ok(())
    }

    pub async fn update_node_tasks(&self, node_id: Uuid, task_count: usize) -> Result<()> {
        let mut tasks = self.node_tasks.write().await;
        tasks.insert(node_id, task_count);
        Ok(())
    }

    pub async fn select_node(&self, task: &TaskSpec) -> Result<Option<Uuid>> {
        let nodes = self.nodes.read().await;
        let health = self.node_health.read().await;
        let tasks = self.node_tasks.read().await;
        
        let healthy_nodes: Vec<_> = nodes.iter()
            .filter(|(id, _)| matches!(health.get(id), Some(NodeHealth::Healthy)))
            .map(|(_, node)| node.clone())
            .collect();
            
        if healthy_nodes.is_empty() {
            return Ok(None);
        }

        let selected = match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                let mut round = self.current_round.write().await;
                *round = (*round + 1) % healthy_nodes.len();
                Some(healthy_nodes[*round].node_id)
            }
            LoadBalanceStrategy::LeastLoaded => {
                healthy_nodes.iter()
                    .min_by_key(|node| tasks.get(&node.node_id).unwrap_or(&usize::MAX))
                    .map(|node| node.node_id)
            }
            LoadBalanceStrategy::DataAware => {
                self.data_scheduler.select_best_node(task, &healthy_nodes)
                    .await?
            }
            LoadBalanceStrategy::Custom(_) => {
                Some(healthy_nodes[0].node_id)
            }
        };

        if selected.is_some() {
            self.metrics.increment_counter("scheduler.tasks.assigned", 1).await;
        }

        Ok(selected)
    }

    pub async fn update_node_health(&self, node_id: Uuid, health: NodeHealth) -> Result<()> {
        let mut node_health = self.node_health.write().await;
        node_health.insert(node_id, health);
        Ok(())
    }
} 