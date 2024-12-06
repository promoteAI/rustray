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
        results.push((task_id, task.block_position));
    }
    
    // 等待并收集所有结果
    let mut final_result = Matrix::new(1000, 1000);
    for (task_id, position) in results {
        let result = head.get_task_result(task_id).await?;
        if let TaskResult::Completed(data) = result {
            let block: Matrix = bincode::deserialize(&data)?;
            merge_result(&mut final_result, block, position);
        }
    }
    
    println!("Matrix multiplication completed!");
    Ok(())
}

struct BlockPosition {
    row: usize,
    col: usize,
}

struct MatrixTask {
    name: String,
    args: Vec<Vec<u8>>,
    priority: i32,
    block_position: BlockPosition,
}

fn split_matrix_multiply_task(a: Matrix, b: Matrix) -> Vec<MatrixTask> {
    let mut tasks = Vec::new();
    let block_size = 250; // 将1000x1000的矩阵分成16个250x250的块
    
    for i in 0..(a.rows / block_size) {
        for j in 0..(b.cols / block_size) {
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
            
            let task = MatrixTask {
                name: "matrix_multiply".to_string(),
                args: vec![
                    bincode::serialize(&block_a).unwrap(),
                    bincode::serialize(&block_b).unwrap(),
                ],
                priority: 0,
                block_position: BlockPosition {
                    row: i * block_size,
                    col: j * block_size,
                },
            };
            tasks.push(task);
        }
    }
    tasks
}

fn merge_result(final_result: &mut Matrix, block_result: Matrix, position: BlockPosition) {
    final_result.set_block(position.row, position.col, &block_result);
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