use std::sync::Arc;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::common::{
    object_store::{ObjectId, ObjectStore},
    WorkerResources, TaskSpec, NodeInfo,
};
use crate::worker::WorkerNode;

pub struct DataAwareScheduler {
    object_store: Arc<ObjectStore>,
    worker_data_locality: Arc<RwLock<HashMap<String, Vec<ObjectId>>>>,
    worker_resources: Arc<RwLock<HashMap<String, WorkerResources>>>,
    worker_tasks: Arc<RwLock<HashMap<String, Vec<TaskSpec>>>>,
    locality_weight: f64,
    resource_weight: f64,
    load_weight: f64,
}

#[derive(Debug)]
struct WorkerScore {
    worker_id: String,
    data_locality_score: f64,
    resource_availability_score: f64,
    load_score: f64,
    total_score: f64,
}

impl DataAwareScheduler {
    pub fn new(object_store: Arc<ObjectStore>) -> Self {
        Self {
            object_store,
            worker_data_locality: Arc::new(RwLock::new(HashMap::new())),
            worker_resources: Arc::new(RwLock::new(HashMap::new())),
            worker_tasks: Arc::new(RwLock::new(HashMap::new())),
            locality_weight: 0.4,
            resource_weight: 0.4,
            load_weight: 0.2,
        }
    }

    pub async fn register_worker(&self, worker: &WorkerNode) -> Result<()> {
        let worker_id = worker.node_info.node_id.to_string();
        
        // 初始化工作节点的数据局部性信息
        self.worker_data_locality.write().await
            .insert(worker_id.clone(), Vec::new());
        
        // 初始化工作节点的资源信息
        self.worker_resources.write().await
            .insert(worker_id.clone(), WorkerResources::default());
        
        // 初始化工作节点的任务列表
        self.worker_tasks.write().await
            .insert(worker_id.clone(), Vec::new());
        
        info!("Registered worker: {}", worker_id);
        Ok(())
    }

    pub async fn select_worker(&self, task: &TaskSpec, input_objects: &[ObjectId]) -> Result<String> {
        let workers = self.worker_resources.read().await;
        if workers.is_empty() {
            return Err(anyhow!("No available workers"));
        }

        let mut best_worker: Option<WorkerScore> = None;
        let locality_map = self.worker_data_locality.read().await;
        let tasks_map = self.worker_tasks.read().await;

        for (worker_id, resources) in workers.iter() {
            // 计算数据局部性分数
            let locality_score = self.calculate_data_locality_score(
                worker_id,
                input_objects,
                &locality_map
            );

            // 计算资源可用性分数
            let resource_score = self.calculate_resource_score(
                resources,
                &task.required_resources
            );

            // 计算负载分数
            let load_score = self.calculate_load_score(
                worker_id,
                &tasks_map
            );

            // 计算总分
            let total_score = 
                self.locality_weight * locality_score +
                self.resource_weight * resource_score +
                self.load_weight * load_score;

            let score = WorkerScore {
                worker_id: worker_id.clone(),
                data_locality_score: locality_score,
                resource_availability_score: resource_score,
                load_score,
                total_score,
            };

            if let Some(ref current_best) = best_worker {
                if score.total_score > current_best.total_score {
                    best_worker = Some(score);
                }
            } else {
                best_worker = Some(score);
            }
        }

        match best_worker {
            Some(worker) => {
                info!(
                    "Selected worker {} for task (locality: {:.2}, resources: {:.2}, load: {:.2})",
                    worker.worker_id,
                    worker.data_locality_score,
                    worker.resource_availability_score,
                    worker.load_score
                );
                Ok(worker.worker_id)
            }
            None => Err(anyhow!("Could not find suitable worker"))
        }
    }

    fn calculate_data_locality_score(
        &self,
        worker_id: &str,
        input_objects: &[ObjectId],
        locality_map: &HashMap<String, Vec<ObjectId>>,
    ) -> f64 {
        if input_objects.is_empty() {
            return 1.0;
        }

        let worker_objects = locality_map.get(worker_id).unwrap_or(&Vec::new());
        let local_objects = input_objects.iter()
            .filter(|obj| worker_objects.contains(obj))
            .count();

        local_objects as f64 / input_objects.len() as f64
    }

    fn calculate_resource_score(
        &self,
        available: &WorkerResources,
        required: &crate::common::TaskRequiredResources,
    ) -> f64 {
        let mut scores = Vec::new();

        // CPU 评分
        if let Some(req_cpu) = required.cpu {
            if available.cpu_total > 0.0 {
                scores.push(available.cpu_available / req_cpu);
            }
        }

        // 内存评分
        if let Some(req_mem) = required.memory {
            if available.memory_total > 0 {
                scores.push(available.memory_available as f64 / req_mem as f64);
            }
        }

        // GPU 评分
        if let Some(req_gpu) = required.gpu {
            if available.gpu_total > 0 {
                scores.push(available.gpu_available as f64 / req_gpu as f64);
            }
        }

        if scores.is_empty() {
            1.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        }
    }

    fn calculate_load_score(&self, worker_id: &str, tasks_map: &HashMap<String, Vec<TaskSpec>>) -> f64 {
        let tasks = tasks_map.get(worker_id).unwrap_or(&Vec::new());
        let task_count = tasks.len();

        if task_count == 0 {
            1.0
        } else {
            1.0 / (task_count as f64 + 1.0)
        }
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

    pub async fn add_task(&self, worker_id: &str, task: TaskSpec) -> Result<()> {
        let mut tasks = self.worker_tasks.write().await;
        if let Some(worker_tasks) = tasks.get_mut(worker_id) {
            worker_tasks.push(task);
        } else {
            warn!("Attempted to add task to unknown worker: {}", worker_id);
        }
        Ok(())
    }

    pub async fn remove_task(&self, worker_id: &str, task_id: &str) -> Result<()> {
        let mut tasks = self.worker_tasks.write().await;
        if let Some(worker_tasks) = tasks.get_mut(worker_id) {
            worker_tasks.retain(|task| task.task_id != task_id);
        }
        Ok(())
    }
} 