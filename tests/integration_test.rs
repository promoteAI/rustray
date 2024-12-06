//! 集成测试
//! 
//! 测试整个系统的端到端功能

use rustray::common::{Matrix, ObjectStore};
use rustray::scheduler::TaskScheduler;
use rustray::worker::WorkerNode;
use rustray::head::HeadNode;
use rustray::task::{Task, TaskResult};
use std::time::Duration;
use tokio;
use anyhow::Result;

#[tokio::test]
async fn test_end_to_end_matrix_multiply() -> Result<()> {
    // 1. 启动系统组件
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    let worker1 = WorkerNode::new("127.0.0.1".to_string(), 8001);
    let worker2 = WorkerNode::new("127.0.0.1".to_string(), 8002);

    // 2. 创建测试数据
    let matrix_a = Matrix::random(1000, 1000);
    let matrix_b = Matrix::random(1000, 1000);

    // 3. 提交任务
    let task = Task::new_matrix_multiply(matrix_a, matrix_b);
    let task_id = head.submit_task(task).await?;

    // 4. 等待结果
    let result = loop {
        if let Some(result) = head.get_task_result(&task_id).await? {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    // 5. 验证结果
    assert!(result.is_success());
    let matrix_c = result.get_matrix()?;
    assert_eq!(matrix_c.rows(), 1000);
    assert_eq!(matrix_c.cols(), 1000);

    Ok(())
}

#[tokio::test]
async fn test_fault_tolerance() -> Result<()> {
    // 1. 启动系统组件
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    let worker1 = WorkerNode::new("127.0.0.1".to_string(), 8001);
    let worker2 = WorkerNode::new("127.0.0.1".to_string(), 8002);

    // 2. 提交多个任务
    let mut task_ids = Vec::new();
    for i in 0..10 {
        let task = Task::new_compute_intensive(i);
        task_ids.push(head.submit_task(task).await?);
    }

    // 3. 模拟节点故障
    tokio::time::sleep(Duration::from_secs(1)).await;
    worker1.simulate_failure();

    // 4. 验证任务是否被重新调度
    for task_id in task_ids {
        let result = loop {
            if let Some(result) = head.get_task_result(&task_id).await? {
                break result;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        assert!(result.is_success());
    }

    Ok(())
}

#[tokio::test]
async fn test_load_balancing() -> Result<()> {
    // 1. 启动系统组件
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    let worker1 = WorkerNode::new("127.0.0.1".to_string(), 8001);
    let worker2 = WorkerNode::new("127.0.0.1".to_string(), 8002);
    let worker3 = WorkerNode::new("127.0.0.1".to_string(), 8003);

    // 2. 提交大量任务
    let mut task_ids = Vec::new();
    for i in 0..100 {
        let task = Task::new_compute_intensive(i);
        task_ids.push(head.submit_task(task).await?);
    }

    // 3. 等待所有任务完成
    for task_id in task_ids {
        let result = loop {
            if let Some(result) = head.get_task_result(&task_id).await? {
                break result;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        assert!(result.is_success());
    }

    // 4. 验证负载均衡
    let stats1 = worker1.get_stats().await?;
    let stats2 = worker2.get_stats().await?;
    let stats3 = worker3.get_stats().await?;

    // 检查任务分布是否相对均匀
    let total_tasks = stats1.completed_tasks + stats2.completed_tasks + stats3.completed_tasks;
    let avg_tasks = total_tasks / 3;
    
    assert!((stats1.completed_tasks as i64 - avg_tasks as i64).abs() <= 10);
    assert!((stats2.completed_tasks as i64 - avg_tasks as i64).abs() <= 10);
    assert!((stats3.completed_tasks as i64 - avg_tasks as i64).abs() <= 10);

    Ok(())
}

#[tokio::test]
async fn test_data_locality() -> Result<()> {
    // 1. 启动系统组件
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    let worker1 = WorkerNode::new("127.0.0.1".to_string(), 8001);
    let worker2 = WorkerNode::new("127.0.0.1".to_string(), 8002);

    // 2. 在worker1上存储数据
    let data = vec![0u8; 1024 * 1024]; // 1MB
    let data_id = worker1.store_data(data.clone()).await?;

    // 3. 提交使用该数据的任务
    let task = Task::new_data_processing(data_id);
    let task_id = head.submit_task(task).await?;

    // 4. 等待任务完成
    let result = loop {
        if let Some(result) = head.get_task_result(&task_id).await? {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    // 5. 验证任务是否在worker1上执行
    assert!(result.is_success());
    let execution_info = result.get_execution_info()?;
    assert_eq!(execution_info.worker_id, worker1.get_id());

    Ok(())
}

#[tokio::test]
async fn test_resource_management() -> Result<()> {
    // 1. 启动系统组件
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    let worker = WorkerNode::new("127.0.0.1".to_string(), 8001);

    // 2. 设置资源限制
    worker.set_resource_limits(ResourceLimits {
        cpu_cores: 2,
        memory_mb: 1024,
        storage_mb: 1024,
    }).await?;

    // 3. 提���超出资源限制的任务
    let task = Task::new_resource_intensive(ResourceRequirements {
        cpu_cores: 4,
        memory_mb: 2048,
        storage_mb: 2048,
    });
    let task_id = head.submit_task(task).await?;

    // 4. 验证任务是否被拒绝
    tokio::time::sleep(Duration::from_secs(1)).await;
    let result = head.get_task_result(&task_id).await?;
    assert!(result.is_some());
    assert!(result.unwrap().is_rejected());

    // 5. 提交符合资源限制的任务
    let task = Task::new_resource_intensive(ResourceRequirements {
        cpu_cores: 1,
        memory_mb: 512,
        storage_mb: 512,
    });
    let task_id = head.submit_task(task).await?;

    // 6. 验证任务是否成功执行
    let result = loop {
        if let Some(result) = head.get_task_result(&task_id).await? {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert!(result.is_success());

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    // 1. 启动系统组件
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    let worker = WorkerNode::new("127.0.0.1".to_string(), 8001);

    // 2. 提交会失败的任务
    let task = Task::new_failing_task();
    let task_id = head.submit_task(task).await?;

    // 3. 验证错误处理
    let result = loop {
        if let Some(result) = head.get_task_result(&task_id).await? {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    assert!(result.is_error());
    let error = result.get_error()?;
    assert!(error.contains("Task failed"));

    // 4. 检查重试机制
    assert_eq!(result.get_retry_count()?, 3); // 默认重试3次

    // 5. 验证系统状态
    assert!(worker.is_healthy().await?);
    assert_eq!(worker.get_failed_task_count().await?, 1);

    Ok(())
}

#[tokio::test]
async fn test_performance_metrics() -> Result<()> {
    // 1. 启动系统组件
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    let worker = WorkerNode::new("127.0.0.1".to_string(), 8001);

    // 2. 提交测试任务
    let task = Task::new_compute_intensive(42);
    let task_id = head.submit_task(task).await?;

    // 3. 等待任务完成
    let result = loop {
        if let Some(result) = head.get_task_result(&task_id).await? {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    // 4. 验证性能指标
    let metrics = result.get_performance_metrics()?;
    
    assert!(metrics.execution_time > Duration::from_millis(0));
    assert!(metrics.cpu_usage > 0.0);
    assert!(metrics.memory_usage > 0);
    assert!(metrics.io_operations > 0);

    // 5. 检查系统级指标
    let system_metrics = head.get_system_metrics().await?;
    
    assert!(system_metrics.total_tasks_completed > 0);
    assert!(system_metrics.avg_task_duration > Duration::from_millis(0));
    assert!(system_metrics.system_load > 0.0);

    Ok(())
} 