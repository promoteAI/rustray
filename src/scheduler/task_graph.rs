//! 任务图模块
//! 
//! 本模块实现了任务依赖关系图的管理，支持：
//! - 任务依赖关系的创建和管理
//! - 任务状态追踪
//! - 任务执行顺序优化
//! - 并行执行调度
//! - 故障恢复和重试

use std::collections::{HashMap, HashSet, VecDeque, BinaryHeap};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use std::cmp::Ordering;

use crate::common::{TaskPriority, TaskSpec, TaskResult};
use crate::metrics::MetricsCollector;

/// 任务节点状态
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    /// 等待依赖任务完成
    Pending,
    /// 就绪可执行
    Ready,
    /// 正在执行
    Running,
    /// 执行完成
    Completed,
    /// 执行失败
    Failed(String),
}

/// 任务节点
#[derive(Debug)]
pub struct TaskNode {
    /// 任务规范
    pub task: TaskSpec,
    /// 当前状态
    pub state: TaskState,
    /// 依赖任务ID列表
    pub dependencies: HashSet<String>,
    /// 依赖此任务的任务ID列表
    pub dependents: HashSet<String>,
    /// 重试次数
    pub retry_count: usize,
    /// 最后一次执行结果
    pub last_result: Option<TaskResult>,
}

/// 任务优先级队列中的元素
#[derive(Debug, Clone)]
struct PrioritizedTask {
    priority: i32,
    timestamp: u64,
    task: TaskSpec,
}

impl Eq for PrioritizedTask {}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.timestamp == other.timestamp
    }
}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // 优先级高的排在前面
        other.priority.cmp(&self.priority)
            .then_with(|| self.timestamp.cmp(&other.timestamp))
    }
}

/// 任务图管理器
pub struct TaskGraph {
    /// 任务节点映射
    nodes: Arc<Mutex<HashMap<String, TaskNode>>>,
    /// 就绪队列
    ready_queue: Arc<Mutex<VecDeque<String>>>,
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 状态更新通道
    status_tx: mpsc::Sender<TaskGraphStatus>,
}

/// 任务图状态
#[derive(Debug, Clone)]
pub struct TaskGraphStatus {
    /// 总任务数
    pub total_tasks: usize,
    /// 等待中的任务数
    pub pending_tasks: usize,
    /// 就绪的任务数
    pub ready_tasks: usize,
    /// 运行中的任务数
    pub running_tasks: usize,
    /// 完成的任务数
    pub completed_tasks: usize,
    /// 失败的任务数
    pub failed_tasks: usize,
}

impl TaskGraph {
    /// 创建新的任务图管理器
    pub fn new(metrics: Arc<MetricsCollector>, status_tx: mpsc::Sender<TaskGraphStatus>) -> Self {
        Self {
            nodes: Arc::new(Mutex::new(HashMap::new())),
            ready_queue: Arc::new(Mutex::new(VecDeque::new())),
            metrics,
            status_tx,
        }
    }

    /// 添加任务到图中
    pub fn add_task(&self, task: TaskSpec, dependencies: Vec<String>) -> Result<(), String> {
        let mut nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let task_id = task.task_id.clone();

        // 创建新任务节点
        let node = TaskNode {
            task,
            state: if dependencies.is_empty() {
                TaskState::Ready
            } else {
                TaskState::Pending
            },
            dependencies: dependencies.into_iter().collect(),
            dependents: HashSet::new(),
            retry_count: 0,
            last_result: None,
        };

        // 更新依赖关系
        for dep_id in node.dependencies.iter() {
            if let Some(dep_node) = nodes.get_mut(dep_id) {
                dep_node.dependents.insert(task_id.clone());
            } else {
                return Err(format!("Dependency task {} not found", dep_id));
            }
        }

        // 如果任务就绪，加入就绪队列
        if node.state == TaskState::Ready {
            let mut ready_queue = self.ready_queue.lock().map_err(|e| e.to_string())?;
            ready_queue.push_back(task_id.clone());
        }

        nodes.insert(task_id.clone(), node);
        
        // 更新指标
        self.metrics.increment_counter("task_graph.tasks.added", 1);
        self.update_status()?;

        info!("Added task {} to graph", task_id);
        Ok(())
    }

    /// 获取下一个可执行的任务
    pub fn get_next_task(&self) -> Option<TaskSpec> {
        let mut ready_queue = self.ready_queue.lock().ok()?;
        let mut nodes = self.nodes.lock().ok()?;

        while let Some(task_id) = ready_queue.pop_front() {
            if let Some(node) = nodes.get_mut(&task_id) {
                if node.state == TaskState::Ready {
                    node.state = TaskState::Running;
                    self.metrics.increment_counter("task_graph.tasks.started", 1);
                    return Some(node.task.clone());
                }
            }
        }

        None
    }

    /// 更新任务状态
    pub fn update_task_state(
        &self,
        task_id: &str,
        state: TaskState,
        result: Option<TaskResult>,
    ) -> Result<(), String> {
        let mut nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        
        if let Some(node) = nodes.get_mut(task_id) {
            let old_state = node.state.clone();
            node.state = state.clone();
            node.last_result = result;

            // 如果任务完成，更新依赖它的任务
            if matches!(state, TaskState::Completed) {
                let mut ready_queue = self.ready_queue.lock().map_err(|e| e.to_string())?;
                
                for dependent_id in node.dependents.iter() {
                    if let Some(dependent_node) = nodes.get_mut(dependent_id) {
                        dependent_node.dependencies.remove(task_id);
                        
                        // 如果所有依赖都完成，将任务设为就绪状态
                        if dependent_node.dependencies.is_empty() 
                            && dependent_node.state == TaskState::Pending 
                        {
                            dependent_node.state = TaskState::Ready;
                            ready_queue.push_back(dependent_id.clone());
                        }
                    }
                }

                self.metrics.increment_counter("task_graph.tasks.completed", 1);
            } else if matches!(state, TaskState::Failed(_)) {
                self.metrics.increment_counter("task_graph.tasks.failed", 1);
            }

            info!(
                "Task {} state changed: {:?} -> {:?}",
                task_id, old_state, state
            );
            
            self.update_status()?;
        }

        Ok(())
    }

    /// 检查是否存在环形依赖
    pub fn check_cycles(&self) -> Result<bool, String> {
        let nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for task_id in nodes.keys() {
            if !visited.contains(task_id) {
                if self.has_cycle(task_id, &nodes, &mut visited, &mut stack)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// 深度优先搜索检测环
    fn has_cycle(
        &self,
        task_id: &str,
        nodes: &HashMap<String, TaskNode>,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> Result<bool, String> {
        visited.insert(task_id.to_string());
        stack.insert(task_id.to_string());

        if let Some(node) = nodes.get(task_id) {
            for dep_id in &node.dependencies {
                if !visited.contains(dep_id) {
                    if self.has_cycle(dep_id, nodes, visited, stack)? {
                        return Ok(true);
                    }
                } else if stack.contains(dep_id) {
                    return Ok(true);
                }
            }
        }

        stack.remove(task_id);
        Ok(false)
    }

    /// 获取任务的关键路径
    pub fn get_critical_path(&self, task_id: &str) -> Result<Vec<String>, String> {
        let nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let mut path = Vec::new();
        let mut current_id = task_id.to_string();

        while let Some(node) = nodes.get(&current_id) {
            path.push(current_id.clone());
            
            // 找到执行时间最长的依赖任务
            if let Some(next_id) = node.dependencies.iter()
                .max_by_key(|dep_id| {
                    nodes.get(*dep_id)
                        .map(|n| n.task.timeout.unwrap_or_default().as_secs())
                        .unwrap_or(0)
                }) 
            {
                current_id = next_id.clone();
            } else {
                break;
            }
        }

        path.reverse();
        Ok(path)
    }

    /// 更新任务图状态
    fn update_status(&self) -> Result<(), String> {
        let nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let ready_queue = self.ready_queue.lock().map_err(|e| e.to_string())?;
        
        let mut pending = 0;
        let mut running = 0;
        let mut completed = 0;
        let mut failed = 0;

        for node in nodes.values() {
            match node.state {
                TaskState::Pending => pending += 1,
                TaskState::Running => running += 1,
                TaskState::Completed => completed += 1,
                TaskState::Failed(_) => failed += 1,
                _ => {}
            }
        }

        let status = TaskGraphStatus {
            total_tasks: nodes.len(),
            pending_tasks: pending,
            ready_tasks: ready_queue.len(),
            running_tasks: running,
            completed_tasks: completed,
            failed_tasks: failed,
        };

        self.status_tx.try_send(status)
            .map_err(|e| e.to_string())?;
        
        Ok(())
    }

    // 添加任务批处理功能
    pub fn batch_add_tasks(&self, tasks: Vec<TaskSpec>) -> Result<(), String> {
        let mut nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let mut ready_queue = self.ready_queue.lock().map_err(|e| e.to_string())?;
        
        for task in tasks {
            let task_id = task.task_id.clone();
            let node = TaskNode {
                task,
                state: TaskState::Ready,
                dependencies: HashSet::new(),
                dependents: HashSet::new(),
                retry_count: 0,
                last_result: None,
            };
            
            nodes.insert(task_id.clone(), node);
            ready_queue.push_back(task_id);
        }
        
        self.metrics.increment_counter("task_graph.tasks.batch_added", tasks.len() as u64)
            .map_err(|e| e.to_string())?;
            
        Ok(())
    }

    // 添加任务恢复功能
    pub fn recover_failed_tasks(&self) -> Result<Vec<String>, String> {
        let mut nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let mut ready_queue = self.ready_queue.lock().map_err(|e| e.to_string())?;
        let mut recovered = Vec::new();

        for (task_id, node) in nodes.iter_mut() {
            if let TaskState::Failed(_) = node.state {
                if node.retry_count < 3 {  // 最大重试次数
                    node.state = TaskState::Ready;
                    node.retry_count += 1;
                    ready_queue.push_back(task_id.clone());
                    recovered.push(task_id.clone());
                }
            }
        }

        if !recovered.is_empty() {
            self.metrics.increment_counter("task_graph.tasks.recovered", recovered.len() as u64)
                .map_err(|e| e.to_string())?;
        }

        Ok(recovered)
    }

    // 添加任务超时处理
    pub fn handle_timeouts(&self) -> Result<Vec<String>, String> {
        let mut nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut timed_out = Vec::new();

        for (task_id, node) in nodes.iter_mut() {
            if let TaskState::Running = node.state {
                if let Some(timeout) = node.task.timeout {
                    if now > timeout.as_secs() {
                        node.state = TaskState::Failed("Task timed out".to_string());
                        timed_out.push(task_id.clone());
                    }
                }
            }
        }

        if !timed_out.is_empty() {
            self.metrics.increment_counter("task_graph.tasks.timed_out", timed_out.len() as u64)
                .map_err(|e| e.to_string())?;
        }

        Ok(timed_out)
    }

    // 添加任务优先级调度
    pub fn get_next_prioritized_task(&self) -> Option<TaskSpec> {
        let mut ready_queue = self.ready_queue.lock().ok()?;
        let nodes = self.nodes.lock().ok()?;
        let mut priority_queue = BinaryHeap::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 将就绪任务按优先级排序
        while let Some(task_id) = ready_queue.pop_front() {
            if let Some(node) = nodes.get(&task_id) {
                if node.state == TaskState::Ready {
                    let priority = node.task.priority.unwrap_or(0);
                    priority_queue.push(PrioritizedTask {
                        priority,
                        timestamp: now,
                        task: node.task.clone(),
                    });
                }
            }
        }

        // 获取优先级最高的任务
        priority_queue.pop().map(|pt| pt.task)
    }

    // 添加任务状态监控
    pub fn monitor_task_states(&self) -> Result<TaskStateStats, String> {
        let nodes = self.nodes.lock().map_err(|e| e.to_string())?;
        let mut stats = TaskStateStats::default();

        for node in nodes.values() {
            match node.state {
                TaskState::Pending => stats.pending += 1,
                TaskState::Ready => stats.ready += 1,
                TaskState::Running => stats.running += 1,
                TaskState::Completed => stats.completed += 1,
                TaskState::Failed(_) => stats.failed += 1,
            }

            // 记录重试统计
            if node.retry_count > 0 {
                stats.retried += 1;
            }
        }

        // 更新指标
        self.metrics.set_gauge("task_graph.tasks.pending", stats.pending as f64)
            .map_err(|e| e.to_string())?;
        self.metrics.set_gauge("task_graph.tasks.ready", stats.ready as f64)
            .map_err(|e| e.to_string())?;
        self.metrics.set_gauge("task_graph.tasks.running", stats.running as f64)
            .map_err(|e| e.to_string())?;
        self.metrics.set_gauge("task_graph.tasks.completed", stats.completed as f64)
            .map_err(|e| e.to_string())?;
        self.metrics.set_gauge("task_graph.tasks.failed", stats.failed as f64)
            .map_err(|e| e.to_string())?;
        self.metrics.set_gauge("task_graph.tasks.retried", stats.retried as f64)
            .map_err(|e| e.to_string())?;

        Ok(stats)
    }
}

#[derive(Debug, Default)]
pub struct TaskStateStats {
    pub pending: usize,
    pub ready: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub retried: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_task(id: &str) -> TaskSpec {
        TaskSpec {
            task_id: id.to_string(),
            function_name: "test_function".to_string(),
            priority: Some(TaskPriority::Normal),
            timeout: Some(Duration::from_secs(10)),
            ..Default::default()
        }
    }

    #[test]
    fn test_task_graph_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let graph = TaskGraph::new(metrics, tx);
        
        assert!(graph.nodes.lock().unwrap().is_empty());
        assert!(graph.ready_queue.lock().unwrap().is_empty());
    }

    #[test]
    fn test_add_task() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let graph = TaskGraph::new(metrics, tx);

        let task = create_test_task("task1");
        let result = graph.add_task(task, vec![]);
        
        assert!(result.is_ok());
        assert_eq!(graph.nodes.lock().unwrap().len(), 1);
        assert_eq!(graph.ready_queue.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_task_dependencies() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let graph = TaskGraph::new(metrics, tx);

        // 添加第一个任务
        let task1 = create_test_task("task1");
        graph.add_task(task1, vec![]).unwrap();

        // 添加依赖于task1的第二个任务
        let task2 = create_test_task("task2");
        graph.add_task(task2, vec!["task1".to_string()]).unwrap();

        let nodes = graph.nodes.lock().unwrap();
        assert!(nodes.get("task2").unwrap().dependencies.contains("task1"));
        assert!(nodes.get("task1").unwrap().dependents.contains("task2"));
    }

    #[test]
    fn test_cycle_detection() {
        let (tx, _rx) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let graph = TaskGraph::new(metrics, tx);

        // 创建循环依赖
        let task1 = create_test_task("task1");
        let task2 = create_test_task("task2");
        let task3 = create_test_task("task3");

        graph.add_task(task1, vec![]).unwrap();
        graph.add_task(task2, vec!["task1".to_string()]).unwrap();
        graph.add_task(task3, vec!["task2".to_string(), "task1".to_string()]).unwrap();

        assert!(!graph.check_cycles().unwrap());
    }
} 