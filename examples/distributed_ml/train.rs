use rustray::task::{Task, TaskResult};
use rustray::head::HeadNode;
use rustray::common::{Matrix, DataSet};
use std::error::Error;
use serde::{Serialize, Deserialize};

/// 模型参数
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelParams {
    weights: Matrix,
    bias: Vec<f64>,
    learning_rate: f64,
}

/// 训练配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrainConfig {
    batch_size: usize,
    epochs: usize,
    learning_rate: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化头节点
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    
    // 加载数据集
    let dataset = DataSet::from_csv("data/mnist.csv")?;
    let (train_data, test_data) = dataset.split_train_test(0.8);
    
    // 配置训练参数
    let config = TrainConfig {
        batch_size: 64,
        epochs: 10,
        learning_rate: 0.01,
    };
    
    // 初始化模型参数
    let model = ModelParams {
        weights: Matrix::random(784, 10),  // MNIST: 784维输入，10个类别
        bias: vec![0.0; 10],
        learning_rate: config.learning_rate,
    };
    
    // 分批训练数据
    let batches = train_data.into_batches(config.batch_size);
    
    println!("Starting distributed training...");
    for epoch in 0..config.epochs {
        let mut epoch_loss = 0.0;
        
        // 并行处理每个批次
        for (i, batch) in batches.iter().enumerate() {
            // 创建训练任务
            let task = Task {
                name: "train_batch".to_string(),
                args: vec![
                    bincode::serialize(&model)?,
                    bincode::serialize(batch)?,
                ],
                priority: 0,
            };
            
            // 提交任务
            let task_id = head.submit_task(task).await?;
            
            // 获取训练结果
            let result = loop {
                if let Some(result) = head.get_task_result(&task_id).await? {
                    break result;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            };
            
            // 更新模型参数
            let batch_result: (ModelParams, f64) = bincode::deserialize(&result.data)?;
            model = batch_result.0;
            epoch_loss += batch_result.1;
            
            if i % 10 == 0 {
                println!("Epoch {}, Batch {}, Loss: {:.4}", epoch, i, epoch_loss / (i + 1) as f64);
            }
        }
        
        // 评估模型
        let eval_task = Task {
            name: "evaluate".to_string(),
            args: vec![
                bincode::serialize(&model)?,
                bincode::serialize(&test_data)?,
            ],
            priority: 1,
        };
        
        let task_id = head.submit_task(eval_task).await?;
        let result = loop {
            if let Some(result) = head.get_task_result(&task_id).await? {
                break result;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        };
        
        let accuracy: f64 = bincode::deserialize(&result.data)?;
        println!("Epoch {} completed, Test Accuracy: {:.2}%", epoch, accuracy * 100.0);
    }
    
    // 保存模型
    let model_data = bincode::serialize(&model)?;
    std::fs::write("model.bin", model_data)?;
    println!("Model saved to model.bin");
    
    Ok(())
}

// Worker节点上运行的训练函数
#[cfg(test)]
mod tests {
    use super::*;
    
    fn train_batch(model: &ModelParams, batch: &DataBatch) -> (ModelParams, f64) {
        let mut new_model = model.clone();
        let mut batch_loss = 0.0;
        
        // 前向传播
        for (x, y) in batch.iter() {
            let output = forward(&new_model, x);
            let loss = cross_entropy_loss(&output, y);
            batch_loss += loss;
            
            // 反向传播
            let gradients = backward(&new_model, &output, x, y);
            
            // 更新参数
            update_params(&mut new_model, &gradients);
        }
        
        (new_model, batch_loss / batch.len() as f64)
    }
    
    fn forward(model: &ModelParams, input: &[f64]) -> Vec<f64> {
        // 实现前向传播
        let mut output = vec![0.0; model.bias.len()];
        for i in 0..model.bias.len() {
            let mut sum = model.bias[i];
            for (j, x) in input.iter().enumerate() {
                sum += model.weights.get(j, i) * x;
            }
            output[i] = softmax(sum);
        }
        output
    }
    
    fn backward(
        model: &ModelParams,
        output: &[f64],
        input: &[f64],
        target: &[f64],
    ) -> Gradients {
        // 实现反向传播
        let mut gradients = Gradients {
            weights: Matrix::zeros(model.weights.rows(), model.weights.cols()),
            bias: vec![0.0; model.bias.len()],
        };
        
        // 计算梯度
        for i in 0..output.len() {
            let error = output[i] - target[i];
            gradients.bias[i] = error;
            
            for (j, x) in input.iter().enumerate() {
                *gradients.weights.get_mut(j, i) = error * x;
            }
        }
        
        gradients
    }
    
    fn update_params(model: &mut ModelParams, gradients: &Gradients) {
        // 更新模型参数
        for i in 0..model.bias.len() {
            model.bias[i] -= model.learning_rate * gradients.bias[i];
            
            for j in 0..model.weights.rows() {
                let w = model.weights.get_mut(j, i);
                *w -= model.learning_rate * gradients.weights.get(j, i);
            }
        }
    }
    
    fn softmax(x: f64) -> f64 {
        x.exp() / (1.0 + x.exp())
    }
    
    fn cross_entropy_loss(output: &[f64], target: &[f64]) -> f64 {
        -target.iter()
            .zip(output.iter())
            .map(|(t, o)| t * o.ln() + (1.0 - t) * (1.0 - o).ln())
            .sum::<f64>()
    }
    
    #[derive(Debug)]
    struct Gradients {
        weights: Matrix,
        bias: Vec<f64>,
    }
} 