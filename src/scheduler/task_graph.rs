use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use tokio::sync::RwLock;
use std::sync::Arc;
use anyhow::Result;

/// 任务ID
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TaskId(Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// 任务状态
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed(String),
}

/// 任务节点
#[derive(Debug)]
pub struct TaskNode {
    pub id: TaskId,
    pub state: TaskState,
    pub dependencies: HashSet<TaskId>,
    pub dependents: HashSet<TaskId>,
    pub priority: i32,
}

/// 任务图管理器
pub struct TaskGraph {
    nodes: RwLock<HashMap<TaskId, Arc<RwLock<TaskNode>>>>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
        }
    }

    /// 添加任务
    pub async fn add_task(&self, dependencies: Vec<TaskId>, priority: i32) -> Result<TaskId> {
        let task_id = TaskId::new();
        let mut deps = HashSet::new();
        
        // 添加依赖
        for dep_id in dependencies {
            if let Some(dep_node) = self.nodes.read().await.get(&dep_id) {
                deps.insert(dep_id.clone());
                // 更新依赖任务的依赖者列表
                dep_node.write().await.dependents.insert(task_id.clone());
            }
        }
        
        // 创建新任务节点
        let node = TaskNode {
            id: task_id.clone(),
            state: TaskState::Pending,
            dependencies: deps,
            dependents: HashSet::new(),
            priority,
        };
        
        // 添加到图中
        self.nodes.write().await.insert(task_id.clone(), Arc::new(RwLock::new(node)));
        
        Ok(task_id)
    }

    /// 获取可执行的任务
    pub async fn get_ready_tasks(&self) -> Vec<TaskId> {
        let mut ready_tasks = Vec::new();
        let nodes = self.nodes.read().await;
        
        for (task_id, node) in nodes.iter() {
            let node = node.read().await;
            if node.state == TaskState::Pending && node.dependencies.is_empty() {
                ready_tasks.push(task_id.clone());
            }
        }
        
        // 按优先级排序
        ready_tasks.sort_by_key(|task_id| {
            -nodes.get(task_id).unwrap().blocking_read().priority
        });
        
        ready_tasks
    }

    /// 更新任务状态
    pub async fn update_task_state(&self, task_id: &TaskId, new_state: TaskState) -> Result<()> {
        if let Some(node) = self.nodes.read().await.get(task_id) {
            let mut node = node.write().await;
            node.state = new_state.clone();
            
            // 如果任务完成，更新依赖它的任务
            if new_state == TaskState::Completed {
                for dependent_id in node.dependents.iter() {
                    if let Some(dependent_node) = self.nodes.read().await.get(dependent_id) {
                        let mut dependent_node = dependent_node.write().await;
                        dependent_node.dependencies.remove(task_id);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// 获取任务状态
    pub async fn get_task_state(&self, task_id: &TaskId) -> Option<TaskState> {
        self.nodes.read().await
            .get(task_id)
            .map(|node| node.read().blocking_read().state.clone())
    }

    /// 获取任务的所有依赖
    pub async fn get_dependencies(&self, task_id: &TaskId) -> Option<HashSet<TaskId>> {
        self.nodes.read().await
            .get(task_id)
            .map(|node| node.read().blocking_read().dependencies.clone())
    }

    /// 检查是否有环
    pub async fn has_cycle(&self) -> bool {
        let nodes = self.nodes.read().await;
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for task_id in nodes.keys() {
            if !visited.contains(task_id) {
                if self.dfs_check_cycle(task_id, &nodes, &mut visited, &mut stack).await {
                    return true;
                }
            }
        }
        false
    }

    async fn dfs_check_cycle(
        &self,
        task_id: &TaskId,
        nodes: &HashMap<TaskId, Arc<RwLock<TaskNode>>>,
        visited: &mut HashSet<TaskId>,
        stack: &mut HashSet<TaskId>,
    ) -> bool {
        visited.insert(task_id.clone());
        stack.insert(task_id.clone());

        if let Some(node) = nodes.get(task_id) {
            let node = node.read().await;
            for dep_id in &node.dependencies {
                if !visited.contains(dep_id) {
                    if self.dfs_check_cycle(dep_id, nodes, visited, stack).await {
                        return true;
                    }
                } else if stack.contains(dep_id) {
                    return true;
                }
            }
        }

        stack.remove(task_id);
        false
    }
} 