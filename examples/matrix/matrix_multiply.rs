use rustray::task::{Task, TaskResult};
use rustray::head::HeadNode;
use rustray::common::Matrix;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化头节点
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    
    // 创建两个大矩阵
    let matrix_a = Matrix::random(1000, 1000);
    let matrix_b = Matrix::random(1000, 1000);
    
    // 将矩阵分块，创建多个子任务
    let tasks = split_matrix_multiply_task(matrix_a, matrix_b);
    
    // 提交任务到分布式系统
    let mut results = Vec::new();
    for task in tasks {
        let task_id = head.submit_task(task).await?;
        results.push(task_id);
    }
    
    // 等待并收集所有结果
    let mut final_result = Matrix::new(1000, 1000);
    for task_id in results {
        let result = head.get_task_result(task_id).await?;
        merge_result(&mut final_result, result);
    }
    
    println!("Matrix multiplication completed!");
    Ok(())
}

fn split_matrix_multiply_task(a: Matrix, b: Matrix) -> Vec<Task> {
    let mut tasks = Vec::new();
    let block_size = 250; // 将1000x1000的矩阵分成16个250x250的块
    
    for i in 0..(a.rows / block_size) {
        for j in 0..(b.cols / block_size) {
            let task = Task {
                name: "matrix_multiply".to_string(),
                args: vec![
                    a.get_block(i * block_size, (i + 1) * block_size),
                    b.get_block(j * block_size, (j + 1) * block_size),
                ],
                priority: 0,
            };
            tasks.push(task);
        }
    }
    tasks
}

fn merge_result(final_result: &mut Matrix, block_result: TaskResult) {
    // 将子任务结果合并到最终结果矩阵中
    // 实际实现需要处理块的位置信息
} 