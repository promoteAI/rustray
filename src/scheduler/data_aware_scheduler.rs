//! 数据感知调度器
//! 
//! 本模块实现了一个智能的数据感知调度系统，可以根据数据位置和资源状态进行任务调度。
//! 主要功能：
//! - 数据局部性优化
//! - 资源感知调度
//! - 负载均衡
//! - 任务优先级处理
//! - 故障恢复

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::common::{NodeInfo, TaskSpec, TaskRequiredResources};
use crate::metrics::collector::MetricsCollector;
use crate::error::{Result, RustRayError};

pub struct DataAwareScheduler {
    node_resources: Arc<RwLock<HashMap<Uuid, TaskRequiredResources>>>,
    data_locality: Arc<RwLock<HashMap<String, Vec<Uuid>>>>,
    metrics: Arc<MetricsCollector>,
}

impl DataAwareScheduler {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self {
            node_resources: Arc::new(RwLock::new(HashMap::new())),
            data_locality: Arc::new(RwLock::new(HashMap::new())),
            metrics,
        }
    }

    pub async fn update_node_resources(
        &self,
        node_id: Uuid,
        resources: TaskRequiredResources,
    ) -> Result<()> {
        let mut node_resources = self.node_resources.write().await;
        node_resources.insert(node_id, resources);
        Ok(())
    }

    pub async fn update_data_locality(
        &self,
        data_id: String,
        node_ids: Vec<Uuid>,
    ) -> Result<()> {
        let mut data_locality = self.data_locality.write().await;
        data_locality.insert(data_id, node_ids);
        Ok(())
    }

    pub async fn select_node(
        &self,
        task: &TaskSpec,
        available_nodes: &[NodeInfo],
    ) -> Result<Option<Uuid>> {
        let node_resources = self.node_resources.read().await;
        let data_locality = self.data_locality.read().await;

        // Filter nodes by resource requirements
        let mut candidate_nodes: Vec<_> = available_nodes.iter()
            .filter(|node| {
                if let Some(resources) = node_resources.get(&node.node_id) {
                    task.required_resources.can_fit(resources)
                } else {
                    false
                }
            })
            .collect();

        if candidate_nodes.is_empty() {
            return Ok(None);
        }

        // Score nodes based on data locality
        let mut node_scores = HashMap::new();
        for node in &candidate_nodes {
            let mut score = 0.0;
            
            // Calculate data locality score
            for data_id in &task.input_data {
                if let Some(locations) = data_locality.get(data_id) {
                    if locations.contains(&node.node_id) {
                        score += 1.0;
                    }
                }
            }

            node_scores.insert(node.node_id, score);
        }

        // Sort nodes by score
        candidate_nodes.sort_by(|a, b| {
            let score_a = node_scores.get(&a.node_id).unwrap_or(&0.0);
            let score_b = node_scores.get(&b.node_id).unwrap_or(&0.0);
            score_b.partial_cmp(score_a).unwrap()
        });

        // Return the node with the highest score
        Ok(Some(candidate_nodes[0].node_id))
    }
} 