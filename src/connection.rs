use tokio::time::{sleep, Duration};
use tokio::net::TcpStream;
use crate::error::{Result, RustRayError};

/// 连接管理器，负责维护与远程节点的连接
pub struct ConnectionManager {
    /// 远程节点的地址
    address: String,
    /// 本地节点的唯一标识符
    node_id: String,
    /// 重试间隔（秒）
    retry_interval: u64,
    /// 最大重试次数
    max_retries: u32,
}

impl ConnectionManager {
    /// 创建新的连接管理器实例
    /// 
    /// # Arguments
    /// * `address` - 远程节点的地址
    /// * `node_id` - 本地节点的唯一标识符
    pub fn new(address: String, node_id: String) -> Self {
        Self {
            address,
            node_id,
            retry_interval: 5,
            max_retries: 3,
        }
    }

    /// 维护与远程节点的连接
    /// 
    /// 定期检查连接状态，在连接断开时进行重试
    pub async fn maintain_connection(&self) -> Result<()> {
        let mut retry_count = 0;
        
        loop {
            match self.check_connection().await {
                Ok(_) => {
                    retry_count = 0;
                    sleep(Duration::from_secs(self.retry_interval)).await;
                }
                Err(e) => {
                    if retry_count >= self.max_retries {
                        return Err(RustRayError::CommunicationError(
                            format!("Failed to maintain connection after {} retries: {}", self.max_retries, e)
                        ));
                    }
                    
                    retry_count += 1;
                    tracing::error!("Connection error (attempt {}/{}): {}", retry_count, self.max_retries, e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// 检查与远程节点的连接状态
    async fn check_connection(&self) -> Result<()> {
        match TcpStream::connect(&self.address).await {
            Ok(_) => {
                tracing::debug!("Connection check successful for node {}", self.node_id);
                Ok(())
            }
            Err(e) => Err(RustRayError::CommunicationError(
                format!("Failed to connect to {}: {}", self.address, e)
            ))
        }
    }
} 