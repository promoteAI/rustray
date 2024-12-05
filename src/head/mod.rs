use crate::common::{NodeInfo, NodeType, TaskSpec};
use crate::grpc::rustray::head_service_server::HeadServiceServer;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;
use uuid::Uuid;
use crate::error::{Result, RustRayError};

/// 头节点，负责管理工作节点和任务分发
pub struct HeadNode {
    /// 节点信息
    pub node_info: NodeInfo,
    /// 已注册的工作节点映射表
    workers: Arc<RwLock<HashMap<Uuid, NodeInfo>>>,
}

impl HeadNode {
    /// 创建新的头节点
    /// 
    /// # Arguments
    /// * `address` - 监听地址
    /// * `port` - 监听端口
    pub fn new(address: String, port: u16) -> Self {
        let node_info = NodeInfo {
            node_id: Uuid::new_v4(),
            node_type: NodeType::Head,
            address,
            port,
        };

        tracing::info!("Creating new head node with ID: {}", node_info.node_id);

        Self {
            node_info,
            workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启动头节点服务器
    pub async fn start_server(&self) -> Result<()> {
        let addr = format!("{}:{}", self.node_info.address, self.node_info.port)
            .parse()
            .map_err(|e| RustRayError::InvalidConfig(format!("Invalid address: {}", e)))?;

        let service = crate::grpc::HeadServiceImpl {
            head: self.clone(),
        };

        tracing::info!("Head node starting on {}", addr);

        Server::builder()
            .add_service(HeadServiceServer::new(service))
            .serve(addr)
            .await
            .map_err(|e| RustRayError::CommunicationError(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// 注册新的工作节点
    /// 
    /// # Arguments
    /// * `worker_info` - 工作节点信息
    pub async fn register_worker(&self, worker_info: NodeInfo) -> Result<()> {
        if worker_info.address.is_empty() {
            tracing::error!("Attempted to register worker with empty address");
            return Err(RustRayError::WorkerRegistrationFailed(
                "Worker address cannot be empty".to_string(),
            ));
        }

        let mut workers = self.workers.write().await;
        
        if workers.contains_key(&worker_info.node_id) {
            tracing::warn!("Worker {} attempted to register again", worker_info.node_id);
            return Err(RustRayError::WorkerRegistrationFailed(
                format!("Worker {} already registered", worker_info.node_id)
            ));
        }

        if workers.len() >= 1000 {
            tracing::error!("Maximum number of workers reached, rejecting registration");
            return Err(RustRayError::ResourceNotAvailable(
                "Maximum number of workers reached".to_string(),
            ));
        }

        tracing::info!("Registering new worker: {}", worker_info.node_id);
        workers.insert(worker_info.node_id, worker_info);
        Ok(())
    }

    /// 提交任务到工作节点
    /// 
    /// # Arguments
    /// * `task` - 任务规范
    pub async fn submit_task(&self, task: TaskSpec) -> Result<()> {
        let workers = self.workers.read().await;
        
        if workers.is_empty() {
            tracing::error!("No workers available to execute task {}", task.task_id);
            return Err(RustRayError::ResourceNotAvailable(
                "No workers available".to_string(),
            ));
        }

        if let Some(worker) = workers.values().next() {
            tracing::info!("Submitting task {} to worker {}", task.task_id, worker.node_id);
            Ok(())
        } else {
            tracing::error!("Failed to find available worker for task {}", task.task_id);
            Err(RustRayError::WorkerNotFound("No worker available".to_string()))
        }
    }

    /// 获取已注册的工作节点数量
    pub async fn get_worker_count(&self) -> usize {
        self.workers.read().await.len()
    }
}

impl Clone for HeadNode {
    fn clone(&self) -> Self {
        Self {
            node_info: self.node_info.clone(),
            workers: self.workers.clone(),
        }
    }
} 