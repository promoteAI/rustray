use std::sync::Arc;
use tokio::sync::RwLock;
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
use tokio::sync::broadcast;

const DEFAULT_CHUNK_SIZE: usize = 64 * 1024 * 1024; // 64MB
const DEFAULT_CACHE_SIZE: usize = 1024 * 1024 * 1024; // 1GB
const DEFAULT_REPLICA_COUNT: usize = 3;
const REBALANCE_INTERVAL: Duration = Duration::from_secs(300); // 5分钟
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectId(Uuid);

#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    size: usize,
    chunks: Vec<ChunkInfo>,
    created_at: Instant,
    last_accessed: Arc<RwLock<Instant>>,
    access_count: Arc<RwLock<usize>>,
    owner: String,
    data_type: String,
    compression: CompressionType,
    replicas: Vec<String>,
    version: u64,
    checksum: String,
    ttl: Option<Duration>,
    tags: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct ChunkInfo {
    chunk_id: String,
    size: usize,
    offset: usize,
    checksum: String,
    locations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ObjectStore {
    // 分布式缓存层 (类似 Redis Cluster)
    local_cache: Arc<DashMap<ObjectId, CacheEntry>>,
    metadata: Arc<DashMap<ObjectId, ObjectMetadata>>,
    chunks: Arc<DashMap<String, Bytes>>,
    
    // 一致性哈希环 (用于数据分片)
    hash_ring: Arc<RwLock<ConsistentHashRing<String>>>,
    
    // 节点管理 (类似 Kubernetes)
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
    node_health: Arc<DashMap<String, NodeHealth>>,
    
    // 数据复制和恢复 (类似 HDFS)
    replication_manager: Arc<ReplicationManager>,
    recovery_manager: Arc<RecoveryManager>,
    
    // 任务调度 (类似 Spark)
    task_scheduler: Arc<TaskScheduler>,
    
    // 配置
    config: StoreConfig,
    
    // 事件通知
    event_sender: broadcast::Sender<StoreEvent>,
}

#[derive(Debug, Clone)]
struct NodeHealth {
    last_heartbeat: Instant,
    status: NodeStatus,
    load: f64,
    available_space: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum NodeStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

#[derive(Debug)]
struct ReplicationManager {
    pending_replications: Arc<DashMap<String, ReplicationTask>>,
    replication_queue: Arc<RwLock<BTreeMap<Instant, String>>>,
}

#[derive(Debug)]
struct RecoveryManager {
    recovery_tasks: Arc<DashMap<String, RecoveryTask>>,
    failed_nodes: Arc<RwLock<HashSet<String>>>,
}

#[derive(Debug)]
struct TaskScheduler {
    pending_tasks: Arc<RwLock<Vec<StoreTask>>>,
    running_tasks: Arc<DashMap<String, StoreTask>>,
    completed_tasks: Arc<DashMap<String, TaskResult>>,
}

#[derive(Debug, Clone)]
enum StoreEvent {
    ObjectCreated(ObjectId),
    ObjectDeleted(ObjectId),
    ObjectAccessed(ObjectId),
    NodeJoined(String),
    NodeLeft(String),
    ReplicationStarted(String),
    ReplicationCompleted(String),
    RecoveryStarted(String),
    RecoveryCompleted(String),
}

impl ObjectStore {
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

        // 启动后台任务
        store.start_background_tasks().await?;
        
        Ok(store)
    }

    async fn start_background_tasks(&self) -> Result<()> {
        // 1. 启动数据平衡任务
        self.start_rebalance_task().await?;
        
        // 2. 启动健康检查任务
        self.start_health_check_task().await?;
        
        // 3. 启动复制任务处理器
        self.start_replication_processor().await?;
        
        // 4. 启动恢复任务处理器
        self.start_recovery_processor().await?;
        
        // 5. 启动垃圾回收任务
        self.start_garbage_collector().await?;
        
        Ok(())
    }

    async fn start_rebalance_task(&self) -> Result<()> {
        let store = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(REBALANCE_INTERVAL);
            loop {
                interval.tick().await;
                if let Err(e) = store.rebalance_data().await {
                    error!("Data rebalancing failed: {}", e);
                }
            }
        });
        Ok(())
    }

    async fn start_health_check_task(&self) -> Result<()> {
        let store = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEALTH_CHECK_INTERVAL);
            loop {
                interval.tick().await;
                store.check_node_health().await;
            }
        });
        Ok(())
    }

    async fn rebalance_data(&self) -> Result<()> {
        // 1. 计算当前数据分布
        let distribution = self.calculate_data_distribution().await?;
        
        // 2. 计算理想分布
        let target_distribution = self.calculate_target_distribution().await?;
        
        // 3. 生成迁移计划
        let migration_plan = self.generate_migration_plan(
            &distribution,
            &target_distribution
        ).await?;
        
        // 4. 执行迁移
        self.execute_migration_plan(migration_plan).await?;
        
        Ok(())
    }

    async fn check_node_health(&self) {
        let now = Instant::now();
        let mut unhealthy_nodes = Vec::new();
        
        // 检查所有节点的健康状态
        for entry in self.node_health.iter() {
            let node_id = entry.key();
            let health = entry.value();
            
            if health.last_heartbeat.elapsed() > Duration::from_secs(30) {
                unhealthy_nodes.push(node_id.clone());
            }
        }
        
        // 处理不健康的节点
        for node_id in unhealthy_nodes {
            if let Some(mut health) = self.node_health.get_mut(&node_id) {
                health.status = NodeStatus::Unhealthy;
                
                // 触发数据恢复
                if let Err(e) = self.trigger_data_recovery(&node_id).await {
                    error!("Failed to trigger data recovery for node {}: {}", node_id, e);
                }
            }
        }
    }

    async fn trigger_data_recovery(&self, node_id: &str) -> Result<()> {
        // 1. 查找需要恢复的数据
        let affected_objects = self.find_affected_objects(node_id).await?;
        
        // 2. 为每个受影响的对象创建恢复任务
        for obj_id in affected_objects {
            self.recovery_manager.create_recovery_task(obj_id, node_id).await?;
        }
        
        // 3. 通知其他节点开始恢复过程
        self.event_sender.send(StoreEvent::RecoveryStarted(node_id.to_string()))?;
        
        Ok(())
    }

    pub async fn put(&self, data: Vec<u8>, owner: String, data_type: String) -> Result<ObjectId> {
        let id = ObjectId::new();
        
        // 1. 压缩和分片数据
        let (chunks, compression_type) = self.prepare_data(data).await?;
        
        // 2. 选择存储节点
        let selected_nodes = self.select_storage_nodes(
            chunks.len(),
            self.config.replica_count
        ).await?;
        
        // 3. 创建元数据
        let metadata = self.create_metadata(
            &id,
            &chunks,
            compression_type,
            owner,
            data_type,
            &selected_nodes
        ).await?;
        
        // 4. 存储分片
        self.store_chunks(&chunks, &selected_nodes).await?;
        
        // 5. 更新元数据
        self.metadata.insert(id.clone(), metadata);
        
        // 6. 触发复制
        self.trigger_replication(&id).await?;
        
        // 7. 发送事件通知
        self.event_sender.send(StoreEvent::ObjectCreated(id.clone()))?;
        
        info!("Successfully stored object {}", id.to_string());
        counter!("object_store.puts", 1);
        
        Ok(id)
    }

    async fn prepare_data(&self, data: Vec<u8>) -> Result<(Vec<ChunkInfo>, CompressionType)> {
        // 1. 压缩数据
        let compressed = if data.len() > 1024 {
            (compress_prepend_size(&data), CompressionType::Lz4)
        } else {
            (data, CompressionType::None)
        };
        
        // 2. 分片
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

    async fn select_storage_nodes(
        &self,
        chunk_count: usize,
        replica_count: usize
    ) -> Result<Vec<Vec<String>>> {
        let mut selected = Vec::with_capacity(chunk_count);
        let hash_ring = self.hash_ring.read().await;
        let healthy_nodes: Vec<_> = self.node_health.iter()
            .filter(|entry| entry.value().status == NodeStatus::Healthy)
            .map(|entry| entry.key().clone())
            .collect();
            
        if healthy_nodes.len() < replica_count {
            return Err(anyhow!("Not enough healthy nodes for replication"));
        }
        
        for i in 0..chunk_count {
            let key = format!("chunk_{}", i);
            let mut nodes = hash_ring.get_nodes(&key, replica_count);
            
            // 过滤掉不健康的节点
            nodes.retain(|node| healthy_nodes.contains(node));
            
            if nodes.len() < replica_count {
                // 如果没有足够的节点，从健康节点中随机选择
                use rand::seq::IteratorRandom;
                let mut rng = rand::thread_rng();
                while nodes.len() < replica_count {
                    if let Some(node) = healthy_nodes.iter()
                        .filter(|n| !nodes.contains(n))
                        .choose(&mut rng) {
                        nodes.push(node.clone());
                    }
                }
            }
            
            selected.push(nodes);
        }
        
        Ok(selected)
    }

    async fn trigger_replication(&self, id: &ObjectId) -> Result<()> {
        let metadata = self.metadata.get(id)
            .ok_or_else(|| anyhow!("Object not found"))?;
            
        for chunk in &metadata.chunks {
            let task = ReplicationTask {
                chunk_id: chunk.chunk_id.clone(),
                source_nodes: chunk.locations.clone(),
                target_nodes: vec![], // 将在复制管理器中确定
                status: TaskStatus::Pending,
                created_at: Instant::now(),
            };
            
            self.replication_manager.add_task(task).await?;
        }
        
        Ok(())
    }
}

// 实现辅助结构体
impl ReplicationManager {
    fn new() -> Self {
        Self {
            pending_replications: Arc::new(DashMap::new()),
            replication_queue: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
    
    async fn add_task(&self, task: ReplicationTask) -> Result<()> {
        self.pending_replications.insert(task.chunk_id.clone(), task);
        self.replication_queue.write().await.insert(
            Instant::now(),
            task.chunk_id
        );
        Ok(())
    }
}

impl RecoveryManager {
    fn new() -> Self {
        Self {
            recovery_tasks: Arc::new(DashMap::new()),
            failed_nodes: Arc::new(RwLock::new(HashSet::new())),
        }
    }
    
    async fn create_recovery_task(&self, object_id: ObjectId, node_id: &str) -> Result<()> {
        let task = RecoveryTask {
            object_id,
            failed_node: node_id.to_string(),
            status: TaskStatus::Pending,
            created_at: Instant::now(),
        };
        
        self.recovery_tasks.insert(task.object_id.to_string(), task);
        Ok(())
    }
} 