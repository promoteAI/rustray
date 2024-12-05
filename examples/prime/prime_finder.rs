use rustray::task::{Task, TaskResult};
use rustray::head::HeadNode;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化头节点
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    
    // 设置要搜索的范围
    let start = 1_000_000;
    let end = 10_000_000;
    let chunk_size = 1_000_000; // 每个任务处理100万个数
    
    // 创建多个子任务
    let mut tasks = Vec::new();
    for i in (start..end).step_by(chunk_size) {
        let task = Task {
            name: "find_primes".to_string(),
            args: vec![
                i.to_string(),
                (i + chunk_size).min(end).to_string(),
            ],
            priority: 0,
        };
        tasks.push(task);
    }
    
    // 提交任务到分布式系统
    println!("Submitting tasks to find primes between {} and {}", start, end);
    let mut results = Vec::new();
    for task in tasks {
        let task_id = head.submit_task(task).await?;
        results.push(task_id);
    }
    
    // 收集所有结果
    let mut all_primes = Vec::new();
    for task_id in results {
        if let Some(result) = head.get_task_result(task_id).await? {
            let primes: Vec<u64> = bincode::deserialize(&result.data)?;
            all_primes.extend(primes);
        }
    }
    
    println!("Found {} prime numbers", all_primes.len());
    println!("Largest prime found: {}", all_primes.last().unwrap_or(&0));
    Ok(())
}

// Worker节点上运行的素数查找函数
#[cfg(test)]
mod tests {
    fn is_prime(n: u64) -> bool {
        if n <= 1 {
            return false;
        }
        if n <= 3 {
            return true;
        }
        if n % 2 == 0 || n % 3 == 0 {
            return false;
        }
        
        let sqrt_n = (n as f64).sqrt() as u64;
        let mut i = 5;
        while i <= sqrt_n {
            if n % i == 0 || n % (i + 2) == 0 {
                return false;
            }
            i += 6;
        }
        true
    }
} 