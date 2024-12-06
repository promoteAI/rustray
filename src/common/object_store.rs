//! 分布式对象存储系统
//! 
//! 本模块实现了一个高可用、可扩展的分布式对象存储系统。主要特性包括：
//! - 数据分片和复制
//! - 自动负载均衡
//! - 故障检测和恢复
//! - 缓存管理
//! - 垃圾回收
//! 
//! # 架构概述
//! 
//! ```text
//! +----------------+     +----------------+     +----------------+
//! |   客户端API    |     |    元数据管理   |     |   数据节点管理  |
//! +----------------+     +----------------+     +----------------+
//!         |                     |                     |
//! +----------------+     +----------------+     +----------------+
//! |   对象管理器    |     |    复制管理器   |     |   恢复管理器   |
//! +----------------+     +----------------+     +----------------+
//!         |                     |                     |
//! +----------------+     +----------------+     +----------------+
//! |   本地缓存      |     |    一致性哈希   |     |   健康检查    |
//! +----------------+     +----------------+     +----------------+
//! ```

use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use std::collections::{HashMap, HashSet, BTreeMap};
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use uuid::Uuid;
use tracing::{info, warn, error};
use metrics::{counter, gauge};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use dashmap::DashMap;
use bytes::Bytes;
use consistent_hash_ring::ConsistentHashRing;

// 系统常量配置
const DEFAULT_CHUNK_SIZE: usize = 64 * 1024 * 1024; // 64MB
const DEFAULT_CACHE_SIZE: usize = 1024 * 1024 * 1024; // 1GB
const DEFAULT_REPLICA_COUNT: usize = 3;
const REBALANCE_INTERVAL: Duration = Duration::from_secs(300); // 5分钟
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(10);
const MAX_BATCH_SIZE: usize = 1000;
const DEFAULT_TTL: Duration = Duration::from_secs(86400); // 1天

/// 对象标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectId(Uuid);

impl ObjectId {
    /// 创建新的对象ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 从字符串解析对象ID
    pub fn from_string(s: &str) -> Result<Self> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    /// 转换为字符串表示
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

/// 对象元数据
#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    /// 对象大小（字节）
    size: usize,
    /// 数据块信息
    chunks: Vec<ChunkInfo>,
    /// 创建时间
    created_at: Instant,
    /// 最后访问时间
    last_accessed: Arc<RwLock<Instant>>,
    /// 访问计数
    access_count: Arc<RwLock<usize>>,
    /// 所有者
    owner: String,
    /// 数据类型
    data_type: String,
    /// 压缩类型
    compression: CompressionType,
    /// 副本位置
    replicas: Vec<String>,
    /// 版本号
    version: u64,
    /// 校验和
    checksum: String,
    /// 过期时间
    ttl: Option<Duration>,
    /// 标签
    tags: HashMap<String, String>,
}

/// 数据块信息
#[derive(Debug, Clone)]
struct ChunkInfo {
    /// 块ID
    chunk_id: String,
    /// 块大小
    size: usize,
    /// 在对象中的偏移量
    offset: usize,
    /// 校验和
    checksum: String,
    /// 存储位置
    locations: Vec<String>,
}

/// 压缩类型
#[derive(Debug, Clone, PartialEq)]
enum CompressionType {
    /// 不压缩
    None,
    /// LZ4压缩
    Lz4,
    /// Snappy压缩
    Snappy,
}

/// 分布式对象存储
#[derive(Debug)]
pub struct ObjectStore {
    // 核心组件
    local_cache: Arc<DashMap<ObjectId, CacheEntry>>,
    metadata: Arc<DashMap<ObjectId, ObjectMetadata>>,
    chunks: Arc<DashMap<String, Bytes>>,
    
    // 数据分布
    hash_ring: Arc<RwLock<ConsistentHashRing<String>>>,
    
    // 节点管理
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
    node_health: Arc<DashMap<String, NodeHealth>>,
    
    // 数据管理
    replication_manager: Arc<ReplicationManager>,
    recovery_manager: Arc<RecoveryManager>,
    
    // 任务调度
    task_scheduler: Arc<TaskScheduler>,
    
    // 配置
    config: StoreConfig,
    
    // 事件通知
    event_sender: broadcast::Sender<StoreEvent>,
}

impl ObjectStore {
    /// 创建新的对象存储实例
    pub async fn new(config: StoreConfig) -> Result<Self> {
        let (event_sender, _) = broadcast::channel(1000);
        
        let store = Self {
            local_cache: Arc::new(DashMap::new()),
            metadata: Arc::new(DashMap::new()),
            chunks: Arc::new(DashMap::new()),
            hash_ring: Arc::new(RwLock::new(ConsistentHashRing::new())),
            nodes: Arc::new(RwLock::new(HashMap::new())),
            node_health: Arc::new(DashMap::new()),
            replication_manager: Arc::new(ReplicationManager::new()),
            recovery_manager: Arc::new(RecoveryManager::new()),
            task_scheduler: Arc::new(TaskScheduler::new()),
            config,
            event_sender,
        };

        store.start_background_tasks().await?;
        Ok(store)
    }

    /// 存储对象
    /// 
    /// # Arguments
    /// * `data` - 对象数据
    /// * `owner` - 所有者
    /// * `data_type` - 数据类型
    /// 
    /// # Returns
    /// * `ObjectId` - 存储的对象ID
    pub async fn put(&self, data: Vec<u8>, owner: String, data_type: String) -> Result<ObjectId> {
        let id = ObjectId::new();
        
        // 1. 数据预处理
        let (chunks, compression_type) = self.prepare_data(data).await?;
        
        // 2. 节点选择
        let selected_nodes = self.select_storage_nodes(
            chunks.len(),
            self.config.replica_count
        ).await?;
        
        // 3. 元数据创建
        let metadata = self.create_metadata(
            &id,
            &chunks,
            compression_type,
            owner,
            data_type,
            &selected_nodes
        ).await?;
        
        // 4. 数据存储
        self.store_chunks(&chunks, &selected_nodes).await?;
        
        // 5. 元数据更新
        self.metadata.insert(id.clone(), metadata);
        
        // 6. 触发复制
        self.trigger_replication(&id).await?;
        
        // 7. 发送事件
        self.event_sender.send(StoreEvent::ObjectCreated(id.clone()))?;
        
        info!("Successfully stored object {}", id.to_string());
        counter!("object_store.puts", 1);
        
        Ok(id)
    }

    /// 获取对象
    /// 
    /// # Arguments
    /// * `id` - 对象ID
    /// 
    /// # Returns
    /// * `(Vec<u8>, String)` - (对象数据, 数据类型)
    pub async fn get(&self, id: &ObjectId) -> Result<(Vec<u8>, String)> {
        // 更新访问统计
        self.update_access_stats(id).await?;
        
        // 尝试从缓存获取
        if let Some(data) = self.get_from_cache(id).await? {
            return Ok(data);
        }
        
        // 从存储节点获取
        let data = self.get_from_storage(id).await?;
        
        // 更新缓存
        self.update_cache(id, &data.0).await?;
        
        Ok(data)
    }

    /// 删除对象
    /// 
    /// # Arguments
    /// * `id` - 对象ID
    pub async fn delete(&self, id: &ObjectId) -> Result<()> {
        // 1. 验证对象存在
        let metadata = self.metadata.get(id)
            .ok_or_else(|| anyhow!("Object not found"))?;
            
        // 2. 删除数据块
        for chunk in &metadata.chunks {
            self.delete_chunk(&chunk.chunk_id).await?;
        }
        
        // 3. 删除元数据
        self.metadata.remove(id);
        
        // 4. 清理缓存
        self.local_cache.remove(id);
        
        // 5. 发送事件
        self.event_sender.send(StoreEvent::ObjectDeleted(id.clone()))?;
        
        info!("Successfully deleted object {}", id.to_string());
        counter!("object_store.deletes", 1);
        
        Ok(())
    }
}

// 私有辅助方法
impl ObjectStore {
    async fn prepare_data(&self, data: Vec<u8>) -> Result<(Vec<ChunkInfo>, CompressionType)> {
        // 压缩处理
        let compressed = if data.len() > 1024 {
            (compress_prepend_size(&data), CompressionType::Lz4)
        } else {
            (data, CompressionType::None)
        };
        
        // 分片处理
        let mut chunks = Vec::new();
        let mut offset = 0;
        
        while offset < compressed.0.len() {
            let chunk_size = std::cmp::min(
                DEFAULT_CHUNK_SIZE,
                compressed.0.len() - offset
            );
            
            let chunk_data = compressed.0[offset..offset + chunk_size].to_vec();
            let chunk_id = Uuid::new_v4().to_string();
            let checksum = self.calculate_checksum(&chunk_data);
            
            chunks.push(ChunkInfo {
                chunk_id,
                size: chunk_size,
                offset,
                checksum,
                locations: vec![],
            });
            
            offset += chunk_size;
        }
        
        Ok((chunks, compressed.1))
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_object_lifecycle() {
        // 创建存储实例
        let store = ObjectStore::new(StoreConfig::default()).await.unwrap();
        
        // 测试数据
        let data = b"Hello, World!".to_vec();
        let owner = "test_user".to_string();
        let data_type = "text/plain".to_string();
        
        // 存储对象
        let id = store.put(data.clone(), owner, data_type).await.unwrap();
        
        // 获取对象
        let (retrieved_data, retrieved_type) = store.get(&id).await.unwrap();
        assert_eq!(retrieved_data, data);
        assert_eq!(retrieved_type, "text/plain");
        
        // 删除对象
        store.delete(&id).await.unwrap();
        
        // 验证删除
        assert!(store.get(&id).await.is_err());
    }

    #[test]
    async fn test_data_compression() {
        let store = ObjectStore::new(StoreConfig::default()).await.unwrap();
        
        // 大数据应该被压缩
        let large_data = vec![0u8; 1024 * 1024];
        let (chunks, compression) = store.prepare_data(large_data).await.unwrap();
        assert_eq!(compression, CompressionType::Lz4);
        
        // 小数据不应该被压缩
        let small_data = vec![0u8; 100];
        let (chunks, compression) = store.prepare_data(small_data).await.unwrap();
        assert_eq!(compression, CompressionType::None);
    }

    #[test]
    async fn test_chunk_distribution() {
        let store = ObjectStore::new(StoreConfig::default()).await.unwrap();
        
        // 添加测试节点
        for i in 0..5 {
            store.nodes.write().await.insert(
                format!("node_{}", i),
                NodeInfo {
                    id: format!("node_{}", i),
                    address: format!("127.0.0.1:{}", 8000 + i),
                    available_space: 1024 * 1024 * 1024,
                    total_space: 1024 * 1024 * 1024,
                }
            );
        }
        
        // 测试节点选择
        let selected = store.select_storage_nodes(10, 3).await.unwrap();
        
        // 验证每个分片都有正确数量的副本
        for nodes in selected {
            assert_eq!(nodes.len(), 3);
            
            // 验证没有重复节点
            let mut unique = HashSet::new();
            for node in nodes {
                assert!(unique.insert(node));
            }
        }
    }
} 