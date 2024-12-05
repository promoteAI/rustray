pub mod rustray {
    tonic::include_proto!("rustray");
}

use rustray::*;
use tonic::{Request, Response, Status};
use crate::common::{NodeInfo, TaskSpec, TaskResult};
use crate::head::HeadNode;
use crate::worker::WorkerNode;

pub struct HeadServiceImpl {
    pub head: HeadNode,
}

#[tonic::async_trait]
impl rustray::head_service_server::HeadService for HeadServiceImpl {
    async fn register_worker(
        &self,
        request: Request<RegisterWorkerRequest>,
    ) -> std::result::Result<Response<RegisterWorkerResponse>, Status> {
        let req = request.into_inner();
        
        // 参数验证
        let node_id = uuid::Uuid::parse_str(&req.worker_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid UUID: {}", e)))?;

        if req.address.is_empty() {
            return Err(Status::invalid_argument("Worker address cannot be empty"));
        }

        let node_info = crate::common::NodeInfo {
            node_id,
            node_type: crate::common::NodeType::Worker,
            address: req.address,
            port: req.port as u16,
        };

        match self.head.register_worker(node_info).await {
            Ok(_) => Ok(Response::new(RegisterWorkerResponse {
                success: true,
                message: "Worker registered successfully".to_string(),
            })),
            Err(e) => {
                let status: Status = e.into();
                Err(status)
            }
        }
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> std::result::Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        
        // 验证worker_id
        uuid::Uuid::parse_str(&req.worker_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid UUID: {}", e)))?;

        // 这里可以添加更多的心跳处理逻辑
        Ok(Response::new(HeartbeatResponse { success: true }))
    }
}

pub struct WorkerServiceImpl {
    pub worker: WorkerNode,
}

#[tonic::async_trait]
impl rustray::worker_service_server::WorkerService for WorkerServiceImpl {
    async fn execute_task(
        &self,
        request: Request<ExecuteTaskRequest>,
    ) -> std::result::Result<Response<ExecuteTaskResponse>, Status> {
        let req = request.into_inner();

        // 参数验证
        let task_id = uuid::Uuid::parse_str(&req.task_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid task ID: {}", e)))?;

        if req.function_name.is_empty() {
            return Err(Status::invalid_argument("Function name cannot be empty"));
        }

        let task_spec = crate::common::TaskSpec {
            task_id,
            function_name: req.function_name,
            args: req.args,
            kwargs: req.kwargs,
        };

        match self.worker.execute_task(task_spec).await {
            Ok(result) => Ok(Response::new(ExecuteTaskResponse {
                task_id: result.task_id.to_string(),
                result: result.result,
                error: result.error,
            })),
            Err(e) => Err(e.into()),
        }
    }

    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> std::result::Result<Response<GetStatusResponse>, Status> {
        Ok(Response::new(GetStatusResponse {
            status: Some(WorkerStatus {
                cpu_usage: 0,
                memory_usage: 0,
                task_count: 0,
            }),
        }))
    }
} 