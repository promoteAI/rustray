//! 数据感知调度器
//! 
//! 本模块实现了一个智能的数据感知调度系统，可以根据数据位置和资源状态进行任务调度。
//! 主要功能：
//! - 数据局部性优化
//! - 资源感知调度
//! - 负载均衡
//! - 任务优先级处理
//! - 故障恢复

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::common::{NodeInfo, TaskPriority, TaskRequiredResources, TaskSpec};
use crate::metrics::MetricsCollector;

/// 调度策略
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulingPolicy {
    /// 数据局部性优先
    DataLocality,
    /// 负载均衡优先
    LoadBalancing,
    /// 资源利用率优先
    ResourceUtilization,
    /// 混合策略（权重可配置）
    Hybrid {
        data_locality_weight: f64,
        load_balance_weight: f64,
        resource_util_weight: f64,
    },
}

/// 数据感知调度器
pub struct DataAwareScheduler {
    /// 节点信息映射
    nodes: Arc<Mutex<HashMap<String, NodeInfo>>>,
    /// 节点资源状态
    resources: Arc<Mutex<HashMap<String, TaskRequiredResources>>>,
    /// 数据位置映射
    data_locations: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    /// 任务队列
    task_queue: Arc<Mutex<Vec<TaskSpec>>>,
    /// 调度策略
    policy: SchedulingPolicy,
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 调度器状态发送通道
    status_tx: mpsc::Sender<SchedulerStatus>,
}

/// 调度器状态
#[derive(Debug, Clone)]
pub struct SchedulerStatus {
    /// 总任务数
    pub total_tasks: usize,
    /// 等待中的任务数
    pub pending_tasks: usize,
    /// 运行中的任务数
    pub running_tasks: usize,
    /// 完成的任务数
    pub completed_tasks: usize,
    /// 失败的任务数
    pub failed_tasks: usize,
    /// 平均调度延迟（毫秒）
    pub avg_scheduling_delay_ms: f64,
}

impl DataAwareScheduler {
    /// 创建新的数据感知调度器
    pub fn new(
        policy: SchedulingPolicy,
        metrics: Arc<MetricsCollector>,
        status_tx: mpsc::Sender<SchedulerStatus>,
    ) -> Self {
        Self {
            nodes: Arc::new(Mutex::new(HashMap::new())),
            resources: Arc::new(Mutex::new(HashMap::new())),
            data_locations: Arc::new(Mutex::new(HashMap::new())),
            task_queue: Arc::new(Mutex::new(Vec::new())),
            policy,
            metrics,
            status_tx,
        }
    }

    /// 提交任务到调度器
    pub async fn submit_task(&self, task: TaskSpec) -> Result<(), String> {
        let mut queue = self.task_queue.lock().map_err(|e| e.to_string())?;
        queue.push(task.clone());
        
        // 更新指标
        self.metrics.increment_counter("scheduler.tasks.submitted", 1);
        self.update_status().await?;
        
        info!("Task submitted: {}", task.task_id);
        Ok(())
    }

    /// 选择最佳节点执行任务
    pub async fn select_best_node(&self, task: &TaskSpec) -> Option<String> {
        let nodes = self.nodes.lock().ok()?;
        let resources = self.resources.lock().ok()?;
        let data_locations = self.data_locations.lock().ok()?;

        let mut best_node = None;
        let mut best_score = f64::NEG_INFINITY;

        for (node_id, node_info) in nodes.iter() {
            let score = match self.policy {
                SchedulingPolicy::DataLocality => {
                    self.calculate_data_locality_score(node_id, task, &data_locations)
                }
                SchedulingPolicy::LoadBalancing => {
                    self.calculate_load_balance_score(node_id, &resources)
                }
                SchedulingPolicy::ResourceUtilization => {
                    self.calculate_resource_utilization_score(node_id, task, &resources)
                }
                SchedulingPolicy::Hybrid {
                    data_locality_weight,
                    load_balance_weight,
                    resource_util_weight,
                } => {
                    let data_score = self.calculate_data_locality_score(node_id, task, &data_locations);
                    let load_score = self.calculate_load_balance_score(node_id, &resources);
                    let resource_score = self.calculate_resource_utilization_score(node_id, task, &resources);
                    
                    data_score * data_locality_weight +
                    load_score * load_balance_weight +
                    resource_score * resource_util_weight
                }
            };

            if score > best_score {
                best_score = score;
                best_node = Some(node_id.clone());
            }
        }

        best_node
    }

    /// 计算数据局部性得分
    fn calculate_data_locality_score(
        &self,
        node_id: &str,
        task: &TaskSpec,
        data_locations: &HashMap<String, HashSet<String>>,
    ) -> f64 {
        let mut local_data_count = 0;
        let mut total_data_count = 0;

        // 统计任务输入数据在节点上的分布
        for data_id in task.args.iter() {
            if let Some(locations) = data_locations.get(&data_id.to_string()) {
                total_data_count += 1;
                if locations.contains(node_id) {
                    local_data_count += 1;
                }
            }
        }

        if total_data_count == 0 {
            return 0.0;
        }

        local_data_count as f64 / total_data_count as f64
    }

    /// 计算负载均衡得分
    fn calculate_load_balance_score(
        &self,
        node_id: &str,
        resources: &HashMap<String, TaskRequiredResources>,
    ) -> f64 {
        if let Some(node_resources) = resources.get(node_id) {
            // 简单使用CPU和内存利用率的平均值作为负载指标
            let cpu_usage = node_resources.cpu.unwrap_or(0.0);
            let memory_usage = node_resources.memory.unwrap_or(0) as f64;
            
            1.0 - (cpu_usage + memory_usage) / 2.0
        } else {
            0.0
        }
    }

    /// 计算资源利用率得分
    fn calculate_resource_utilization_score(
        &self,
        node_id: &str,
        task: &TaskSpec,
        resources: &HashMap<String, TaskRequiredResources>,
    ) -> f64 {
        if let Some(node_resources) = resources.get(node_id) {
            let required = &task.required_resources;
            
            // 检查是否满足资源要求
            let cpu_fit = required.cpu.map_or(true, |req| {
                node_resources.cpu.map_or(false, |avail| avail >= req)
            });
            
            let memory_fit = required.memory.map_or(true, |req| {
                node_resources.memory.map_or(false, |avail| avail >= req)
            });
            
            let gpu_fit = required.gpu.map_or(true, |req| {
                node_resources.gpu.map_or(false, |avail| avail >= req)
            });

            if !cpu_fit || !memory_fit || !gpu_fit {
                return 0.0;
            }

            // 计算资源匹配度
            let mut score = 1.0;
            if let (Some(req), Some(avail)) = (required.cpu, node_resources.cpu) {
                score *= req / avail;
            }
            if let (Some(req), Some(avail)) = (required.memory, node_resources.memory) {
                score *= req as f64 / avail as f64;
            }
            if let (Some(req), Some(avail)) = (required.gpu, node_resources.gpu) {
                score *= req as f64 / avail as f64;
            }

            score
        } else {
            0.0
        }
    }

    /// 更新调度器状态
    async fn update_status(&self) -> Result<(), String> {
        let queue = self.task_queue.lock().map_err(|e| e.to_string())?;
        
        let status = SchedulerStatus {
            total_tasks: queue.len(),
            pending_tasks: queue.iter().filter(|t| matches!(t.priority, Some(TaskPriority::Normal))).count(),
            running_tasks: queue.iter().filter(|t| matches!(t.priority, Some(TaskPriority::High))).count(),
            completed_tasks: queue.iter().filter(|t| matches!(t.priority, Some(TaskPriority::Low))).count(),
            failed_tasks: 0,
            avg_scheduling_delay_ms: 0.0,
        };

        self.status_tx.send(status).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let scheduler = DataAwareScheduler::new(
            SchedulingPolicy::DataLocality,
            metrics,
            tx,
        );
        
        assert!(scheduler.nodes.lock().unwrap().is_empty());
        assert!(scheduler.task_queue.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_task_submission() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let scheduler = DataAwareScheduler::new(
            SchedulingPolicy::DataLocality,
            metrics,
            tx,
        );

        let task = TaskSpec {
            task_id: "test_task".to_string(),
            priority: Some(TaskPriority::Normal),
            ..Default::default()
        };

        let result = scheduler.submit_task(task).await;
        assert!(result.is_ok());
        assert_eq!(scheduler.task_queue.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_node_selection() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let scheduler = DataAwareScheduler::new(
            SchedulingPolicy::Hybrid {
                data_locality_weight: 0.4,
                load_balance_weight: 0.3,
                resource_util_weight: 0.3,
            },
            metrics,
            tx,
        );

        // 添加测试节点
        {
            let mut nodes = scheduler.nodes.lock().unwrap();
            nodes.insert(
                "node1".to_string(),
                NodeInfo {
                    node_id: uuid::Uuid::new_v4(),
                    node_type: crate::common::NodeType::Worker,
                    address: "localhost".to_string(),
                    port: 8000,
                },
            );
        }

        // 添加测试资源
        {
            let mut resources = scheduler.resources.lock().unwrap();
            resources.insert(
                "node1".to_string(),
                TaskRequiredResources {
                    cpu: Some(4.0),
                    memory: Some(8192),
                    gpu: Some(1),
                },
            );
        }

        let task = TaskSpec {
            task_id: "test_task".to_string(),
            required_resources: TaskRequiredResources {
                cpu: Some(2.0),
                memory: Some(4096),
                gpu: None,
            },
            ..Default::default()
        };

        let selected_node = scheduler.select_best_node(&task).await;
        assert!(selected_node.is_some());
        assert_eq!(selected_node.unwrap(), "node1");
    }
} 