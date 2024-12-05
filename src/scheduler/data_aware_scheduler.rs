use std::collections::HashMap;
use anyhow::Result;
use crate::common::object_store::{ObjectId, ObjectStore};
use crate::worker::WorkerNode;
use std::sync::Arc;

/// 数据位置感知的调度策略
pub struct DataAwareScheduler {
    object_store: Arc<ObjectStore>,
    workers: HashMap<String, WorkerNode>,
}

impl DataAwareScheduler {
    pub fn new(object_store: Arc<ObjectStore>) -> Self {
        Self {
            object_store,
            workers: HashMap::new(),
        }
    }

    /// 注册工作节点
    pub fn register_worker(&mut self, worker: WorkerNode) {
        self.workers.insert(worker.node_info.node_id.clone(), worker);
    }

    /// 为任务选择最佳的工作节点
    pub async fn select_worker(&self, task_inputs: &[ObjectId]) -> Result<String> {
        let mut node_scores = HashMap::new();
        
        // 计算每个节点的数据局部性分数
        for worker_id in self.workers.keys() {
            let mut score = 0.0;
            
            // 检查每个输入对象
            for obj_id in task_inputs {
                if let Some(locations) = self.object_store.get_locations(obj_id).await {
                    if locations.contains(worker_id) {
                        // 本地数据加高分
                        score += 1.0;
                    } else {
                        // 远程数据根据网络距离计算分数
                        score += self.calculate_network_score(worker_id, &locations);
                    }
                }
            }
            
            // 考虑节点负载
            if let Some(worker) = self.workers.get(worker_id) {
                let load_penalty = worker.get_load_factor().await;
                score /= 1.0 + load_penalty;
            }
            
            node_scores.insert(worker_id.clone(), score);
        }
        
        // 选择得分最高的节点
        node_scores.into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(node_id, _)| node_id)
            .ok_or_else(|| anyhow::anyhow!("No available workers"))
    }

    /// 计算网络距离分数
    fn calculate_network_score(&self, worker_id: &str, data_locations: &[String]) -> f64 {
        // TODO: 实现更复杂的网络拓扑感知
        // 当前简单实现：假设所有远程访问成本相同
        0.1
    }

    /// 预测任务执行时间
    pub async fn predict_execution_time(&self, worker_id: &str, task_inputs: &[ObjectId]) -> f64 {
        let mut time = 0.0;
        
        // 基础执行时间（可以基于历史数据学习）
        time += 1.0;
        
        // 数据传输时间
        for obj_id in task_inputs {
            if let Some(locations) = self.object_store.get_locations(obj_id).await {
                if !locations.contains(worker_id) {
                    // 需要传输数据
                    if let Some(size) = self.object_store.get_object_size(obj_id).await {
                        // 假设网络带宽为100MB/s
                        time += size as f64 / (100.0 * 1024.0 * 1024.0);
                    }
                }
            }
        }
        
        time
    }

    /// 获取节点的资源使用情况
    pub async fn get_worker_resources(&self, worker_id: &str) -> Option<WorkerResources> {
        self.workers.get(worker_id).map(|worker| {
            WorkerResources {
                cpu_usage: worker.get_cpu_usage(),
                memory_usage: worker.get_memory_usage(),
                network_bandwidth: worker.get_network_bandwidth(),
            }
        })
    }
}

/// 工作节点资源信息
#[derive(Debug, Clone)]
pub struct WorkerResources {
    pub cpu_usage: f64,      // CPU使用率 (0-1)
    pub memory_usage: f64,   // 内存使用率 (0-1)
    pub network_bandwidth: f64, // 可用网络带宽 (bytes/s)
} 