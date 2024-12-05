use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::common::object_store::ObjectId;

#[derive(Debug, Clone)]
pub struct TaskNode {
    pub task_id: String,
    pub function_name: String,
    pub args: Vec<ObjectId>,  // 输入对象引用
    pub outputs: Vec<ObjectId>,  // 输出对象引用
    pub state: TaskState,
    pub worker_id: Option<String>,
    pub dependencies: HashSet<String>,  // 依赖任务的ID
    pub dependents: HashSet<String>,    // 依赖此任务的任务ID
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Ready,
    Running,
    Completed,
    Failed(String),
}

pub struct TaskGraph {
    nodes: Arc<RwLock<HashMap<String, TaskNode>>>,
    object_to_task: Arc<RwLock<HashMap<ObjectId, String>>>,  // 对象到任务的映射
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            object_to_task: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_task(
        &self,
        function_name: String,
        args: Vec<ObjectId>,
        outputs: Vec<ObjectId>,
    ) -> String {
        let task_id = Uuid::new_v4().to_string();
        let mut dependencies = HashSet::new();

        // 查找参数对象的生产者任务
        for arg in &args {
            if let Some(producer_id) = self.object_to_task.read().await.get(arg) {
                dependencies.insert(producer_id.clone());
            }
        }

        let node = TaskNode {
            task_id: task_id.clone(),
            function_name,
            args,
            outputs: outputs.clone(),
            state: if dependencies.is_empty() {
                TaskState::Ready
            } else {
                TaskState::Pending
            },
            worker_id: None,
            dependencies,
            dependents: HashSet::new(),
        };

        // 更新任务图
        let mut nodes = self.nodes.write().await;
        let mut object_map = self.object_to_task.write().await;

        // 更新依赖关系
        for dep_id in &node.dependencies {
            if let Some(dep_node) = nodes.get_mut(dep_id) {
                dep_node.dependents.insert(task_id.clone());
            }
        }

        // 记录输出对象的生产者
        for output in outputs {
            object_map.insert(output, task_id.clone());
        }

        nodes.insert(task_id.clone(), node);
        task_id
    }

    pub async fn get_ready_tasks(&self) -> Vec<String> {
        let nodes = self.nodes.read().await;
        nodes.values()
            .filter(|node| node.state == TaskState::Ready)
            .map(|node| node.task_id.clone())
            .collect()
    }

    pub async fn mark_task_completed(&self, task_id: &str) -> Vec<String> {
        let mut newly_ready = Vec::new();
        let mut nodes = self.nodes.write().await;

        if let Some(node) = nodes.get_mut(task_id) {
            node.state = TaskState::Completed;

            // 检查依赖此任务的任务是否可以执行
            for dependent_id in &node.dependents {
                if let Some(dependent_node) = nodes.get_mut(dependent_id) {
                    let all_deps_completed = dependent_node.dependencies.iter()
                        .all(|dep_id| {
                            nodes.get(dep_id)
                                .map(|n| n.state == TaskState::Completed)
                                .unwrap_or(false)
                        });

                    if all_deps_completed && dependent_node.state == TaskState::Pending {
                        dependent_node.state = TaskState::Ready;
                        newly_ready.push(dependent_id.clone());
                    }
                }
            }
        }

        newly_ready
    }

    pub async fn get_task_info(&self, task_id: &str) -> Option<TaskNode> {
        self.nodes.read().await.get(task_id).cloned()
    }

    pub async fn get_object_lineage(&self, object_id: &ObjectId) -> Vec<String> {
        let mut lineage = Vec::new();
        let nodes = self.nodes.read().await;
        
        if let Some(task_id) = self.object_to_task.read().await.get(object_id) {
            let mut current_id = task_id.clone();
            while let Some(node) = nodes.get(&current_id) {
                lineage.push(current_id.clone());
                if let Some(first_dep) = node.dependencies.iter().next() {
                    current_id = first_dep.clone();
                } else {
                    break;
                }
            }
        }
        
        lineage
    }

    pub async fn get_task_dependencies(&self, task_id: &str) -> Option<HashSet<String>> {
        self.nodes.read().await
            .get(task_id)
            .map(|node| node.dependencies.clone())
    }
} 