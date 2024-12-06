//! 负载均衡器模块
//! 
//! 本模块实现了一个智能的负载均衡系统，支持：
//! - 多种负载均衡策略
//! - 动态资源权重调整
//! - 节点健康检查
//! - 自适应负载调节
//! - 性能监控和统计

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::common::{NodeInfo, TaskRequiredResources, TaskSpec};
use crate::metrics::MetricsCollector;

/// 负载均衡策略
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalanceStrategy {
    /// 轮询调度
    RoundRobin,
    /// 最少连接
    LeastConnections,
    /// 加权轮询
    WeightedRoundRobin,
    /// 加权最少连接
    WeightedLeastConnections,
    /// 响应时间加权
    ResponseTimeWeighted,
    /// 资源利用率加权
    ResourceUtilizationWeighted,
}

/// 节点健康状态
#[derive(Debug, Clone, PartialEq)]
pub enum NodeHealth {
    /// 健康
    Healthy,
    /// 亚健康（性能下降）
    Degraded(String),
    /// 不健康
    Unhealthy(String),
}

/// 节点状态
#[derive(Debug, Clone)]
pub struct NodeState {
    /// 节点信息
    pub info: NodeInfo,
    /// 健康状态
    pub health: NodeHealth,
    /// 当前连接数
    pub connections: usize,
    /// 平均响应时间（毫秒）
    pub avg_response_time: f64,
    /// 资源使用情况
    pub resources: TaskRequiredResources,
    /// 权重
    pub weight: f64,
}

/// 负载均衡器
pub struct LoadBalancer {
    /// 负载均衡策略
    strategy: LoadBalanceStrategy,
    /// 节点状态映射
    nodes: Arc<Mutex<HashMap<String, NodeState>>>,
    /// 当前轮询索引
    current_index: Arc<Mutex<usize>>,
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 状态更新通道
    status_tx: mpsc::Sender<LoadBalancerStatus>,
}

/// 负载均衡器状态
#[derive(Debug, Clone)]
pub struct LoadBalancerStatus {
    /// 总节点数
    pub total_nodes: usize,
    /// 健康节点数
    pub healthy_nodes: usize,
    /// 亚健康节点数
    pub degraded_nodes: usize,
    /// 不健康节点数
    pub unhealthy_nodes: usize,
    /// 总连接数
    pub total_connections: usize,
    /// 平均响应时间
    pub avg_response_time: f64,
}

impl LoadBalancer {
    /// 创建新的负载均衡器
    pub fn new(
        strategy: LoadBalanceStrategy,
        metrics: Arc<MetricsCollector>,
        status_tx: mpsc::Sender<LoadBalancerStatus>,
    ) -> Self {
        Self {
            strategy,
            nodes: Arc::new(Mutex::new(HashMap::new())),
            current_index: Arc::new(Mutex::new(0)),
            metrics,
            status_tx,
        }
    }

    /// 注册新节点
    pub fn register_node(&self, node: NodeInfo) -> Result<(), String> {
        let mut nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let node_id = node.node_id.to_string();

        let state = NodeState {
            info: node,
            health: NodeHealth::Healthy,
            connections: 0,
            avg_response_time: 0.0,
            resources: TaskRequiredResources::default(),
            weight: 1.0,
        };

        nodes.insert(node_id.clone(), state);
        
        // 更新指标
        self.metrics.increment_counter("load_balancer.nodes.registered", 1);
        self.update_status()?;

        info!("Registered node: {}", node_id);
        Ok(())
    }

    /// 选择最佳节点处理任务
    pub fn select_node(&self, task: &TaskSpec) -> Option<String> {
        let nodes = self.nodes.lock().ok()?;
        if nodes.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                self.select_round_robin(&nodes)
            }
            LoadBalanceStrategy::LeastConnections => {
                self.select_least_connections(&nodes)
            }
            LoadBalanceStrategy::WeightedRoundRobin => {
                self.select_weighted_round_robin(&nodes)
            }
            LoadBalanceStrategy::WeightedLeastConnections => {
                self.select_weighted_least_connections(&nodes)
            }
            LoadBalanceStrategy::ResponseTimeWeighted => {
                self.select_response_time_weighted(&nodes)
            }
            LoadBalanceStrategy::ResourceUtilizationWeighted => {
                self.select_resource_weighted(&nodes, task)
            }
        }
    }

    /// 更新节点状态
    pub fn update_node_state(
        &self,
        node_id: &str,
        health: NodeHealth,
        connections: Option<usize>,
        response_time: Option<f64>,
        resources: Option<TaskRequiredResources>,
    ) -> Result<(), String> {
        let mut nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        
        if let Some(state) = nodes.get_mut(node_id) {
            let old_health = state.health.clone();
            state.health = health.clone();
            
            if let Some(conn) = connections {
                state.connections = conn;
            }
            
            if let Some(rt) = response_time {
                state.avg_response_time = rt;
            }
            
            if let Some(res) = resources {
                state.resources = res;
            }

            // 根据健康状态调整权重
            state.weight = match health {
                NodeHealth::Healthy => 1.0,
                NodeHealth::Degraded(_) => 0.5,
                NodeHealth::Unhealthy(_) => 0.0,
            };

            // 更新指标
            match health {
                NodeHealth::Healthy => {
                    self.metrics.increment_counter("load_balancer.nodes.healthy", 1);
                }
                NodeHealth::Degraded(_) => {
                    self.metrics.increment_counter("load_balancer.nodes.degraded", 1);
                }
                NodeHealth::Unhealthy(_) => {
                    self.metrics.increment_counter("load_balancer.nodes.unhealthy", 1);
                }
            }

            info!(
                "Node {} health changed: {:?} -> {:?}",
                node_id, old_health, health
            );
            
            self.update_status()?;
        }

        Ok(())
    }

    // 私有辅助方法

    /// 轮询选择
    fn select_round_robin(&self, nodes: &HashMap<String, NodeState>) -> Option<String> {
        let healthy_nodes: Vec<_> = nodes.iter()
            .filter(|(_, state)| state.health == NodeHealth::Healthy)
            .collect();

        if healthy_nodes.is_empty() {
            return None;
        }

        let mut index = self.current_index.lock().ok()?;
        *index = (*index + 1) % healthy_nodes.len();
        Some(healthy_nodes[*index].0.clone())
    }

    /// 最少连接选择
    fn select_least_connections(&self, nodes: &HashMap<String, NodeState>) -> Option<String> {
        nodes.iter()
            .filter(|(_, state)| state.health == NodeHealth::Healthy)
            .min_by_key(|(_, state)| state.connections)
            .map(|(id, _)| id.clone())
    }

    /// 加权轮询选择
    fn select_weighted_round_robin(&self, nodes: &HashMap<String, NodeState>) -> Option<String> {
        let weighted_nodes: Vec<_> = nodes.iter()
            .filter(|(_, state)| state.health == NodeHealth::Healthy)
            .collect();

        if weighted_nodes.is_empty() {
            return None;
        }

        let total_weight: f64 = weighted_nodes.iter()
            .map(|(_, state)| state.weight)
            .sum();

        let mut index = self.current_index.lock().ok()?;
        let position = (*index as f64 * total_weight) / weighted_nodes.len() as f64;
        
        let mut sum = 0.0;
        for (id, state) in weighted_nodes {
            sum += state.weight;
            if sum >= position {
                *index = (*index + 1) % weighted_nodes.len();
                return Some(id.clone());
            }
        }

        None
    }

    /// 加权最少连接选择
    fn select_weighted_least_connections(&self, nodes: &HashMap<String, NodeState>) -> Option<String> {
        nodes.iter()
            .filter(|(_, state)| state.health == NodeHealth::Healthy)
            .min_by(|a, b| {
                let score_a = a.1.connections as f64 / a.1.weight;
                let score_b = b.1.connections as f64 / b.1.weight;
                score_a.partial_cmp(&score_b).unwrap()
            })
            .map(|(id, _)| id.clone())
    }

    /// 响应时间加权选择
    fn select_response_time_weighted(&self, nodes: &HashMap<String, NodeState>) -> Option<String> {
        nodes.iter()
            .filter(|(_, state)| state.health == NodeHealth::Healthy)
            .min_by(|a, b| {
                let score_a = a.1.avg_response_time / a.1.weight;
                let score_b = b.1.avg_response_time / b.1.weight;
                score_a.partial_cmp(&score_b).unwrap()
            })
            .map(|(id, _)| id.clone())
    }

    /// 资源利用率加权选择
    fn select_resource_weighted(
        &self,
        nodes: &HashMap<String, NodeState>,
        task: &TaskSpec,
    ) -> Option<String> {
        nodes.iter()
            .filter(|(_, state)| {
                state.health == NodeHealth::Healthy &&
                self.check_resource_requirements(&state.resources, &task.required_resources)
            })
            .max_by(|a, b| {
                let score_a = self.calculate_resource_score(&a.1.resources);
                let score_b = self.calculate_resource_score(&b.1.resources);
                score_a.partial_cmp(&score_b).unwrap()
            })
            .map(|(id, _)| id.clone())
    }

    /// 检查资源要求是否满足
    fn check_resource_requirements(
        &self,
        available: &TaskRequiredResources,
        required: &TaskRequiredResources,
    ) -> bool {
        if let Some(req_cpu) = required.cpu {
            if available.cpu.unwrap_or(0.0) < req_cpu {
                return false;
            }
        }

        if let Some(req_mem) = required.memory {
            if available.memory.unwrap_or(0) < req_mem {
                return false;
            }
        }

        if let Some(req_gpu) = required.gpu {
            if available.gpu.unwrap_or(0) < req_gpu {
                return false;
            }
        }

        true
    }

    /// 计算资源得分
    fn calculate_resource_score(&self, resources: &TaskRequiredResources) -> f64 {
        let mut score = 0.0;
        let mut count = 0;

        if let Some(cpu) = resources.cpu {
            score += cpu;
            count += 1;
        }

        if let Some(memory) = resources.memory {
            score += memory as f64;
            count += 1;
        }

        if let Some(gpu) = resources.gpu {
            score += gpu as f64;
            count += 1;
        }

        if count == 0 {
            0.0
        } else {
            score / count as f64
        }
    }

    /// 更新负载均衡器状态
    fn update_status(&self) -> Result<(), String> {
        let nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut total_connections = 0;
        let mut total_response_time = 0.0;
        let mut response_time_count = 0;

        for state in nodes.values() {
            match state.health {
                NodeHealth::Healthy => healthy += 1,
                NodeHealth::Degraded(_) => degraded += 1,
                NodeHealth::Unhealthy(_) => unhealthy += 1,
            }

            total_connections += state.connections;
            
            if state.avg_response_time > 0.0 {
                total_response_time += state.avg_response_time;
                response_time_count += 1;
            }
        }

        let status = LoadBalancerStatus {
            total_nodes: nodes.len(),
            healthy_nodes: healthy,
            degraded_nodes: degraded,
            unhealthy_nodes: unhealthy,
            total_connections,
            avg_response_time: if response_time_count > 0 {
                total_response_time / response_time_count as f64
            } else {
                0.0
            },
        };

        self.status_tx.try_send(status)
            .map_err(|e| e.to_string())?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_node(id: &str) -> NodeInfo {
        NodeInfo {
            node_id: Uuid::new_v4(),
            node_type: crate::common::NodeType::Worker,
            address: "localhost".to_string(),
            port: 8000,
        }
    }

    #[test]
    fn test_load_balancer_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let lb = LoadBalancer::new(
            LoadBalanceStrategy::RoundRobin,
            metrics,
            tx,
        );
        
        assert!(lb.nodes.lock().unwrap().is_empty());
    }

    #[test]
    fn test_node_registration() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let lb = LoadBalancer::new(
            LoadBalanceStrategy::RoundRobin,
            metrics,
            tx,
        );

        let node = create_test_node("node1");
        let result = lb.register_node(node);
        
        assert!(result.is_ok());
        assert_eq!(lb.nodes.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_round_robin_selection() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let lb = LoadBalancer::new(
            LoadBalanceStrategy::RoundRobin,
            metrics,
            tx,
        );

        // 注册两个节点
        let node1 = create_test_node("node1");
        let node2 = create_test_node("node2");
        
        lb.register_node(node1).unwrap();
        lb.register_node(node2).unwrap();

        // 测试轮询选择
        let task = TaskSpec::default();
        let first = lb.select_node(&task);
        let second = lb.select_node(&task);
        
        assert!(first.is_some());
        assert!(second.is_some());
        assert_ne!(first, second);
    }

    #[test]
    fn test_node_health_update() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let lb = LoadBalancer::new(
            LoadBalanceStrategy::RoundRobin,
            metrics,
            tx,
        );

        let node = create_test_node("node1");
        let node_id = node.node_id.to_string();
        lb.register_node(node).unwrap();

        // 更新节点状态
        let result = lb.update_node_state(
            &node_id,
            NodeHealth::Degraded("High load".to_string()),
            Some(10),
            Some(100.0),
            None,
        );
        
        assert!(result.is_ok());
        
        let nodes = lb.nodes.lock().unwrap();
        let state = nodes.get(&node_id).unwrap();
        assert!(matches!(state.health, NodeHealth::Degraded(_)));
        assert_eq!(state.connections, 10);
        assert_eq!(state.avg_response_time, 100.0);
    }
} 