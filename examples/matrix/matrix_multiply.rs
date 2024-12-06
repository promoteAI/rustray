use rustray::common::{TaskSpec, TaskResult, TaskPriority, Matrix, object_store::ObjectStore};
use rustray::head::{HeadNode, HeadNodeConfig};
use rustray::metrics::MetricsCollector;
use std::error::Error;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use std::collections::HashMap;

struct BlockPosition {
    row: usize,
    col: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化系统组件
    let metrics = Arc::new(MetricsCollector::new("matrix_multiply".to_string()));
    let object_store = Arc::new(ObjectStore::new("matrix_multiply".to_string()));
    let (status_tx, _status_rx) = mpsc::channel(100);

    let config = HeadNodeConfig {
        address: "127.0.0.1".to_string(),
        port: 8000,
        heartbeat_timeout: Duration::from_secs(5),
        resource_monitor_interval: Duration::from_secs(10),
        scheduling_strategy: Default::default(),
    };

    let head = HeadNode::new(config, object_store, metrics.clone(), status_tx);

    // 创建两个大矩阵
    let matrix_a = Matrix::random(1000, 1000);
    let matrix_b = Matrix::random(1000, 1000);

    // 将矩阵分块，创建多个子任务
    let tasks = split_matrix_multiply_task(matrix_a, matrix_b);

    // 提交任务到分布式系统
    println!("Submitting {} tasks...", tasks.len());

    let mut results = Vec::new();
    for (task_id, task_spec, block_position) in tasks {
        head.submit_task(task_spec).await?;
        results.push((task_id, block_position));
    }

    // 等待并收集所有结果
    let mut final_result = Matrix::new(1000, 1000);
    for (task_id, position) in results {
        if let Some(result) = head.get_task_result(&task_id).await? {
            match result {
                TaskResult::Completed(data) => {
                    let block: Matrix = bincode::deserialize(&data)?;
                    merge_result(&mut final_result, block, &position);
                    println!("Merged block at position ({}, {})", position.row, position.col);
                }
                TaskResult::Failed(err) => {
                    eprintln!("Task failed at position ({}, {}): {}", position.row, position.col, err);
                }
                _ => {}
            }
        }
    }

    println!("Matrix multiplication completed!");
    Ok(())
}

fn split_matrix_multiply_task(a: Matrix, b: Matrix) -> Vec<(Uuid, TaskSpec, BlockPosition)> {
    let mut tasks = Vec::new();
    let block_size = 250; // 将1000x1000的矩阵分成16个250x250的块

    for i in 0..(a.rows / block_size) {
        for j in 0..(b.cols / block_size) {
            // 获取矩阵块
            let block_a = a.get_block(
                i * block_size, 
                (i + 1) * block_size,
                0,
                a.cols
            );
            let block_b = b.get_block(
                0,
                b.rows,
                j * block_size,
                (j + 1) * block_size
            );

            // 序列化矩阵块
            let arg_a = bincode::serialize(&block_a).unwrap();
            let arg_b = bincode::serialize(&block_b).unwrap();

            // ��建任务ID
            let task_id = Uuid::new_v4();

            // 准备任务参数
            let args = vec![arg_a, arg_b];

            // 创建任务规格
            let task = TaskSpec {
                task_id,
                function_name: "matrix_multiply".to_string(),
                kwargs: HashMap::new(),
                args,
                workflow_id: None,
                cache_key: None,
                data_dependencies: Vec::new(),
                required_resources: Default::default(),
                timeout: Some(Duration::from_secs(300)),
                retry_strategy: Default::default(),
                priority: Some(TaskPriority::Normal),
            };

            let block_position = BlockPosition {
                row: i * block_size,
                col: j * block_size,
            };

            tasks.push((task_id, task, block_position));
        }
    }
    tasks
}

fn merge_result(final_result: &mut Matrix, block_result: Matrix, position: &BlockPosition) {
    final_result.set_block(position.row, position.col, &block_result);
}

// 定义用于矩阵乘法的函数
fn matrix_multiply(args: Vec<Vec<u8>>) -> Vec<u8> {
    let block_a: Matrix = bincode::deserialize(&args[0]).unwrap();
    let block_b: Matrix = bincode::deserialize(&args[1]).unwrap();

    let result = block_a.multiply_block(&block_b);

    bincode::serialize(&result).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_multiply() {
        // 创建两个小矩阵进行测试
        let mut a = Matrix::new(2, 2);
        let mut b = Matrix::new(2, 2);
        
        // 设置测试数据
        a.set(0, 0, 1.0); a.set(0, 1, 2.0);
        a.set(1, 0, 3.0); a.set(1, 1, 4.0);
        
        b.set(0, 0, 5.0); b.set(0, 1, 6.0);
        b.set(1, 0, 7.0); b.set(1, 1, 8.0);
        
        // 计算结果
        let result = a.multiply_block(&b);
        
        // 验证结果
        assert_eq!(result.get(0, 0), 19.0); // 1*5 + 2*7
        assert_eq!(result.get(0, 1), 22.0); // 1*6 + 2*8
        assert_eq!(result.get(1, 0), 43.0); // 3*5 + 4*7
        assert_eq!(result.get(1, 1), 50.0); // 3*6 + 4*8
    }
} 