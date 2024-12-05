pub mod load_balancer;

use crate::common::TaskSpec;
use crate::error::Result;
use uuid::Uuid;

/// 任务调度器，负责管理任务的分配和执行
pub struct TaskScheduler {
    /// 负载均衡器
    load_balancer: load_balancer::LoadBalancer,
}

impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new() -> Self {
        Self {
            load_balancer: load_balancer::LoadBalancer::new(
                load_balancer::LoadBalancingStrategy::LeastLoaded
            ),
        }
    }

    /// 调度任务到合适的工作节点
    /// 
    /// # Arguments
    /// * `task` - 要调度的任务
    pub async fn schedule_task(&self, task: TaskSpec) -> Result<Uuid> {
        // TODO: 实现实际的任务调度逻辑
        Ok(task.task_id)
    }
} 