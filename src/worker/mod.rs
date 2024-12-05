use crate::common::{NodeInfo, NodeType, TaskResult, TaskSpec};
use crate::grpc::rustray::worker_service_server::WorkerServiceServer;
use tokio::sync::mpsc;
use tonic::transport::Server;
use uuid::Uuid;
use crate::error::{Result, RustRayError};

/// 工作节点，负责执行具体的计算任务
pub struct WorkerNode {
    /// 节点信息
    pub node_info: NodeInfo,
    /// 任务发送通道
    task_sender: mpsc::Sender<TaskSpec>,
}

impl WorkerNode {
    /// 创建新的工作节点
    /// 
    /// # Arguments
    /// * `address` - 监听地址
    /// * `port` - 监听端口
    /// 
    /// # Returns
    /// 返回工作节点实例和任务接收通道
    pub fn new(address: String, port: u16) -> (Self, mpsc::Receiver<TaskSpec>) {
        let (tx, rx) = mpsc::channel(100);
        
        let node_info = NodeInfo {
            node_id: Uuid::new_v4(),
            node_type: NodeType::Worker,
            address,
            port,
        };

        tracing::info!("Creating new worker node with ID: {}", node_info.node_id);

        (Self {
            node_info,
            task_sender: tx,
        }, rx)
    }

    /// 启动工作节点服务器
    pub async fn start_server(&self) -> Result<()> {
        let addr = format!("{}:{}", self.node_info.address, self.node_info.port)
            .parse()
            .map_err(|e| RustRayError::InvalidConfig(format!("Invalid address: {}", e)))?;

        let service = crate::grpc::WorkerServiceImpl {
            worker: self.clone(),
        };

        tracing::info!("Worker node starting on {}", addr);

        Server::builder()
            .add_service(WorkerServiceServer::new(service))
            .serve(addr)
            .await
            .map_err(|e| RustRayError::CommunicationError(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// 执行任务
    /// 
    /// # Arguments
    /// * `task` - 要执行的任务规范
    pub async fn execute_task(&self, task: TaskSpec) -> Result<TaskResult> {
        tracing::info!("Received task {} for execution", task.task_id);

        // 验证任务参数
        if task.function_name.is_empty() {
            tracing::error!("Task {} has empty function name", task.task_id);
            return Err(RustRayError::TaskExecutionFailed(
                "Function name cannot be empty".to_string(),
            ));
        }

        // 检查资源是否可用
        if !self.check_resources().await? {
            tracing::error!("Insufficient resources to execute task {}", task.task_id);
            return Err(RustRayError::ResourceNotAvailable(
                "Insufficient resources to execute task".to_string(),
            ));
        }

        // 发送任务到处理队列
        match self.task_sender.send(task.clone()).await {
            Ok(_) => tracing::info!("Task {} queued successfully", task.task_id),
            Err(e) => {
                tracing::error!("Failed to queue task {}: {}", task.task_id, e);
                return Err(RustRayError::CommunicationError(format!("Failed to queue task: {}", e)));
            }
        }

        // 返回任务结果
        let result = TaskResult {
            task_id: task.task_id,
            result: Vec::new(),
            error: None,
        };

        tracing::info!("Task {} completed successfully", task.task_id);
        Ok(result)
    }

    /// 检查系统资源是否足够执行任务
    async fn check_resources(&self) -> Result<bool> {
        // TODO: 实现实际的资源检查逻辑
        // 例如：检查CPU使用率、内存使用率等
        Ok(true)
    }
}

impl Clone for WorkerNode {
    fn clone(&self) -> Self {
        Self {
            node_info: self.node_info.clone(),
            task_sender: self.task_sender.clone(),
        }
    }
}
