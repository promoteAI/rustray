use std::collections::HashMap;
use uuid::Uuid;
use crate::common::NodeInfo;
use crate::error::Result;

/// 负载均衡策略
#[derive(Debug, Clone, Copy)]
pub enum LoadBalancingStrategy {
    /// 轮询策略
    RoundRobin,
    /// 最少负载策略
    LeastLoaded,
    /// 随机策略
    Random,
}

/// 负载均衡器，负责在工作节点之间分配任务
pub struct LoadBalancer {
    /// 负载均衡策略
    strategy: LoadBalancingStrategy,
    /// 节点负载信息
    node_loads: HashMap<Uuid, usize>,
}

impl LoadBalancer {
    /// 创建新的负载均衡器
    /// 
    /// # Arguments
    /// * `strategy` - 负载均衡策略
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            node_loads: HashMap::new(),
        }
    }

    /// 选择最适合执行任务的节点
    /// 
    /// # Arguments
    /// * `available_nodes` - 可用的工作节点列表
    pub fn select_node<'a>(&self, available_nodes: &'a [NodeInfo]) -> Result<Option<&'a NodeInfo>> {
        if available_nodes.is_empty() {
            tracing::warn!("No available nodes for task execution");
            return Ok(None);
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                // 简单返回第一个节点，后续实现真正的轮询逻辑
                tracing::debug!("Using round-robin strategy, selecting first node");
                Ok(Some(&available_nodes[0]))
            }
            LoadBalancingStrategy::LeastLoaded => {
                // 找到负载最小的节点
                let selected = available_nodes.iter().min_by_key(|node| {
                    self.node_loads.get(&node.node_id).unwrap_or(&0)
                });
                
                if let Some(node) = selected {
                    tracing::debug!("Selected node {} with least load", node.node_id);
                }
                
                Ok(selected)
            }
            LoadBalancingStrategy::Random => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                
                let selected = available_nodes.choose(&mut rng);
                if let Some(node) = selected {
                    tracing::debug!("Randomly selected node {}", node.node_id);
                }
                
                Ok(selected)
            }
        }
    }

    /// 更新节点的负载信息
    /// 
    /// # Arguments
    /// * `node_id` - 节点ID
    /// * `load` - 当前负载值
    pub fn update_node_load(&mut self, node_id: Uuid, load: usize) {
        self.node_loads.insert(node_id, load);
        tracing::debug!("Updated load for node {}: {}", node_id, load);
    }

    /// 移除节点的负载信息
    /// 
    /// # Arguments
    /// * `node_id` - 要移除的节点ID
    pub fn remove_node(&mut self, node_id: &Uuid) {
        self.node_loads.remove(node_id);
        tracing::debug!("Removed load information for node {}", node_id);
    }
} 