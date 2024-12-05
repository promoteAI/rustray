use std::sync::Arc;
use anyhow::{Result, anyhow};
use crate::common::object_store::ObjectId;
use crate::worker::WorkerNode;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WorkerResources {
    pub cpu_total: f64,
    pub cpu_available: f64,
    pub cpu_usage: f64,
    pub memory_total: usize,
    pub memory_available: usize,
    pub memory_usage: f64,
    pub gpu_total: usize,
    pub gpu_available: usize,
    pub network_bandwidth: f64,
}

impl Default for WorkerResources {
    fn default() -> Self {
        Self {
            cpu_total: num_cpus::get() as f64,
            cpu_available: num_cpus::get() as f64,
            cpu_usage: 0.0,
            memory_total: 1024 * 1024 * 1024, // 1GB
            memory_available: 1024 * 1024 * 1024,
            memory_usage: 0.0,
            gpu_total: 0,
            gpu_available: 0,
            network_bandwidth: 1000.0, // 1Gbps
        }
    }
}

#[allow(dead_code)]
pub struct DataAwareScheduler {
    object_store: Arc<crate::common::object_store::ObjectStore>,
    worker_data_locality: Arc<tokio::sync::RwLock<HashMap<String, Vec<ObjectId>>>>,
    worker_resources: Arc<tokio::sync::RwLock<HashMap<String, WorkerResources>>>,
}

#[allow(dead_code)]
impl DataAwareScheduler {
    pub fn new(object_store: Arc<crate::common::object_store::ObjectStore>) -> Self {
        Self {
            object_store,
            worker_data_locality: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            worker_resources: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub fn register_worker(&self, worker: WorkerNode) {
        let worker_id = worker.node_info.node_id.to_string();
        let resources = WorkerResources::default();
        
        tokio::spawn({
            let worker_resources = self.worker_resources.clone();
            let worker_id = worker_id.clone();
            async move {
                worker_resources.write().await.insert(worker_id, resources);
            }
        });
    }

    pub async fn select_worker(&self, input_objects: &[ObjectId]) -> Result<String> {
        let locality = self.worker_data_locality.read().await;
        let resources = self.worker_resources.read().await;
        
        // 计算每个工作节点的数据局部性分数
        let mut scores = HashMap::new();
        
        for (worker_id, worker_objects) in locality.iter() {
            let mut score = 0.0;
            
            // 数据局部性分数
            for obj_id in input_objects {
                if worker_objects.contains(obj_id) {
                    score += 1.0;
                }
            }
            
            // 资源可用性分数
            if let Some(worker_resources) = resources.get(worker_id) {
                let resource_score = self.calculate_resource_score(worker_resources);
                score *= resource_score;
            }
            
            scores.insert(worker_id.clone(), score);
        }
        
        // 选择得分最高的工作节点
        scores.into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(worker_id, _)| worker_id)
            .ok_or_else(|| anyhow!("No suitable worker found"))
    }

    fn calculate_resource_score(&self, resources: &WorkerResources) -> f64 {
        let cpu_score = resources.cpu_available / resources.cpu_total;
        let memory_score = resources.memory_available as f64 / resources.memory_total as f64;
        let gpu_score = if resources.gpu_total > 0 {
            resources.gpu_available as f64 / resources.gpu_total as f64
        } else {
            1.0
        };
        
        // 加权平均
        0.4 * cpu_score + 0.4 * memory_score + 0.2 * gpu_score
    }

    pub async fn update_worker_resources(
        &self,
        worker_id: &str,
        resources: WorkerResources,
    ) -> Result<()> {
        self.worker_resources.write().await
            .insert(worker_id.to_string(), resources);
        Ok(())
    }

    pub async fn update_data_locality(
        &self,
        worker_id: &str,
        objects: Vec<ObjectId>,
    ) -> Result<()> {
        self.worker_data_locality.write().await
            .insert(worker_id.to_string(), objects);
        Ok(())
    }

    pub async fn get_worker_resources(&self, worker_id: &str) -> Option<WorkerResources> {
        self.worker_resources.read().await
            .get(worker_id)
            .cloned()
    }

    pub async fn get_worker_objects(&self, worker_id: &str) -> Option<Vec<ObjectId>> {
        self.worker_data_locality.read().await
            .get(worker_id)
            .cloned()
    }
} 