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
use crate::metrics::MetricsCollector;
use crate::error::Result;

#[derive(Debug)]
struct NodeScore {
    node_id: Uuid,
    data_locality_score: f64,
    resource_score: f64,
}

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

    pub async fn select_best_node(
        &self,
        task: &TaskSpec,
        available_nodes: &[NodeInfo],
    ) -> Result<Option<Uuid>> {
        let node_resources = self.node_resources.read().await;
        let data_locality = self.data_locality.read().await;

        // 计算每个节点的得分
        let mut node_scores: Vec<NodeScore> = Vec::new();

        for node in available_nodes {
            let resources = match node_resources.get(&node.node_id) {
                Some(r) => r,
                None => continue,
            };

            // 检查资源是否满足要求
            if !task.required_resources.can_fit(resources) {
                continue;
            }

            // 计算数据局部性得分
            let mut locality_score = 0.0;
            let data_deps = &task.data_dependencies;
            if !data_deps.is_empty() {
                for data_id in data_deps {
                    if let Some(nodes) = data_locality.get(data_id) {
                        if nodes.contains(&node.node_id) {
                            locality_score += 1.0;
                        }
                    }
                }
                locality_score /= data_deps.len() as f64;
            }

            // 计算资源得分
            let cpu_score = resources.cpu.map(|cpu| cpu / task.required_resources.cpu.unwrap_or(1.0))
                .unwrap_or(0.0);
            let memory_score = resources.memory.map(|mem| mem as f64 / task.required_resources.memory.unwrap_or(1024) as f64)
                .unwrap_or(0.0);
            let resource_score = (cpu_score + memory_score) / 2.0;

            node_scores.push(NodeScore {
                node_id: node.node_id,
                data_locality_score: locality_score,
                resource_score,
            });
        }

        if node_scores.is_empty() {
            return Ok(None);
        }

        // 根据综合得分选择最佳节点
        let best_node = node_scores.iter()
            .max_by(|a, b| {
                let score_a = a.data_locality_score * 0.7 + a.resource_score * 0.3;
                let score_b = b.data_locality_score * 0.7 + b.resource_score * 0.3;
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|score| score.node_id);

        Ok(best_node)
    }
} 