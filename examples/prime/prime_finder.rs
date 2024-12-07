use rustray::common::{TaskSpec, TaskResult, TaskPriority};
use rustray::head::{HeadNode, HeadNodeConfig};
use rustray::metrics::MetricsCollector;
use rustray::common::object_store::ObjectStore;
use std::error::Error;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use std::collections::HashMap;

const MIN_PRIME: u64 = 2;
const DEFAULT_START: u64 = 1_000_000;
const DEFAULT_END: u64 = 10_000_000;
const DEFAULT_CHUNK_SIZE: u64 = 1_000_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 从环境变量或命令行参数获取配置
    let start = std::env::var("PRIME_START")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_START);
    
    let end = std::env::var("PRIME_END")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_END);
    
    let chunk_size = std::env::var("PRIME_CHUNK_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CHUNK_SIZE);

    // 参数验证
    if start >= end {
        return Err("Start must be less than end".into());
    }
    if chunk_size == 0 {
        return Err("Chunk size must be greater than 0".into());
    }
    
    // 初始化系统组件
    let metrics = Arc::new(MetricsCollector::new("prime_finder".to_string()));
    let object_store = Arc::new(ObjectStore::new("prime_finder".to_string()));
    let (status_tx, _status_rx) = mpsc::channel(100);
    
    let config = HeadNodeConfig {
        address: "127.0.0.1".to_string(),
        port: 8000,
        heartbeat_timeout: Duration::from_secs(5),
        resource_monitor_interval: Duration::from_secs(10),
        scheduling_strategy: Default::default(),
    };
    
    let head = HeadNode::new(config, object_store, metrics.clone(), status_tx);
    
    // 创建多个子任务
    let mut tasks = Vec::new();
    for i in (start..end).step_by(chunk_size as usize) {
        let chunk_end = (i + chunk_size).min(end);
        let task_id = Uuid::new_v4();
        
        // 准备任务参数
        let mut args = HashMap::new();
        args.insert("start".to_string(), i.to_string());
        args.insert("end".to_string(), chunk_end.to_string());
        
        let task = TaskSpec {
            task_id,
            function_name: "find_primes".to_string(),
            kwargs: args,
            args: Vec::new(),  // 空的位置参数
            workflow_id: None,
            cache_key: None,
            data_dependencies: Vec::new(),
            required_resources: Default::default(),
            timeout: Some(Duration::from_secs(300)),
            retry_strategy: Default::default(),
            priority: Some(TaskPriority::Normal),
        };
        tasks.push((task_id, i, chunk_end, task));
    }
    
    // 提交任务到分布式系统
    println!("Finding prime numbers between {} and {}", start, end);
    println!("Using chunk size of {}", chunk_size);
    println!("Submitting {} tasks...", tasks.len());
    
    let timer = Instant::now();
    let mut results = Vec::new();
    let mut completed_chunks = 0;
    let total_chunks = tasks.len();
    
    for (task_id, chunk_start, chunk_end, task) in tasks {
        head.submit_task(task).await?;
        results.push((task_id, chunk_start, chunk_end));
    }
    
    // 收集所有结果
    let mut all_primes = Vec::new();
    for (task_id, chunk_start, chunk_end) in results {
        match head.get_task_result(&task_id).await {
            Ok(Some(result)) => {
                match result {
                    TaskResult::Completed(data) => {
                        let primes: Vec<u64> = bincode::deserialize(&data)?;
                        all_primes.extend(primes);
                        completed_chunks += 1;
                        println!(
                            "Progress: {:.1}% ({}/{}) - Found {} primes in range {} to {}", 
                            (completed_chunks as f64 / total_chunks as f64) * 100.0,
                            completed_chunks,
                            total_chunks,
                            all_primes.len(),
                            chunk_start,
                            chunk_end
                        );
                    }
                    TaskResult::Failed(err) => {
                        eprintln!("Task failed for range {} to {}: {}", chunk_start, chunk_end, err);
                    }
                    _ => {}
                }
            }
            Ok(None) => {
                eprintln!("Task result for {} to {} is not available yet.", chunk_start, chunk_end);
            }
            Err(err) => {
                eprintln!("Error fetching task result for range {} to {}: {}", chunk_start, chunk_end, err);
            }
        }
    }
    
    // 对结果进行排序和去重
    all_primes.sort_unstable();
    all_primes.dedup();
    
    let elapsed = timer.elapsed();
    println!("\nSearch completed in {:.2?}", elapsed);
    println!("Found {} unique prime numbers", all_primes.len());
    
    if !all_primes.is_empty() {
        println!("Smallest prime found: {}", all_primes.first().unwrap());
        println!("Largest prime found: {}", all_primes.last().unwrap());
        
        // 输出一些示例素数
        if all_primes.len() > 10 {
            println!("\nFirst 5 primes found:");
            for &prime in all_primes.iter().take(5) {
                println!("{}", prime);
            }
            println!("\nLast 5 primes found:");
            for &prime in all_primes.iter().rev().take(5) {
                println!("{}", prime);
            }
        }
    }
    
    Ok(())
}

/// 检查一个数是否为素数
fn is_prime(n: u64) -> bool {
    if n < MIN_PRIME {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_prime() {
        // 测试小素数
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(5));
        assert!(is_prime(7));
        assert!(is_prime(11));
        assert!(is_prime(13));
        
        // 测试大素数
        assert!(is_prime(997));
        assert!(!is_prime(1000));
        assert!(is_prime(7919));
        
        // 测试梅森素数
        assert!(is_prime(8191));  // 2^13 - 1
        assert!(is_prime(131071));  // 2^17 - 1
    }
    
    #[test]
    fn test_composite_numbers() {
        // 测试合数
        assert!(!is_prime(4));
        assert!(!is_prime(6));
        assert!(!is_prime(8));
        assert!(!is_prime(9));
        assert!(!is_prime(10));
        assert!(!is_prime(12));
        assert!(!is_prime(100));
        assert!(!is_prime(1000));
    }
    
    #[test]
    fn test_edge_cases() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
    }
} 