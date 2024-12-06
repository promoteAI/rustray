use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use uuid::Uuid;
use tracing::{info, warn, error};
use metrics::{counter, gauge};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use dashmap::DashMap;
use bytes::Bytes;

const DEFAULT_CHUNK_SIZE: usize = 64 * 1024 * 1024; // 64MB
const DEFAULT_CACHE_SIZE: usize = 1024 * 1024 * 1024; // 1GB
const DEFAULT_REPLICA_COUNT: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectId(Uuid);

impl ObjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Self {
        Self(Uuid::parse_str(s).unwrap_or_else(|_| Uuid::new_v4()))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

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
    replicas: Vec<String>, // 存储副本所在的节点ID
}

#[derive(Debug, Clone)]
struct ChunkInfo {
    chunk_id: String,
    size: usize,
    offset: usize,
    checksum: String,
    locations: Vec<String>, // 存储块所在的节点ID
}

#[derive(Debug, Clone, PartialEq)]
enum CompressionType {
    None,
    Lz4,
    Snappy,
}

pub struct ObjectStore {
    local_cache: Arc<DashMap<ObjectId, CacheEntry>>,
    metadata: Arc<DashMap<ObjectId, ObjectMetadata>>,
    chunks: Arc<DashMap<String, Bytes>>,
    remote_nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
    cache_config: CacheConfig,
    shard_manager: ShardManager,
    gc_manager: GarbageCollector,
}

#[derive(Debug)]
struct CacheEntry {
    data: Bytes,
    inserted_at: Instant,
    last_accessed: Instant,
    size: usize,
}

#[derive(Debug, Clone)]
struct CacheConfig {
    max_size: usize,
    ttl: Duration,
    eviction_policy: EvictionPolicy,
}

#[derive(Debug, Clone)]
struct NodeInfo {
    id: String,
    address: String,
    available_space: usize,
    total_space: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum EvictionPolicy {
    LRU,
    LFU,
    FIFO,
}

struct ShardManager {
    chunk_size: usize,
    replica_count: usize,
    placement_strategy: PlacementStrategy,
}

#[derive(Debug, Clone)]
enum PlacementStrategy {
    Random,
    CapacityBased,
    LoadBased,
}

struct GarbageCollector {
    gc_interval: Duration,
    last_gc: Arc<RwLock<Instant>>,
    deleted_objects: Arc<RwLock<HashSet<ObjectId>>>,
}

impl ObjectStore {
    pub fn new() -> Self {
        let cache_config = CacheConfig {
            max_size: DEFAULT_CACHE_SIZE,
            ttl: Duration::from_secs(3600),
            eviction_policy: EvictionPolicy::LRU,
        };

        let shard_manager = ShardManager {
            chunk_size: DEFAULT_CHUNK_SIZE,
            replica_count: DEFAULT_REPLICA_COUNT,
            placement_strategy: PlacementStrategy::CapacityBased,
        };

        let gc_manager = GarbageCollector {
            gc_interval: Duration::from_secs(3600),
            last_gc: Arc::new(RwLock::new(Instant::now())),
            deleted_objects: Arc::new(RwLock::new(HashSet::new())),
        };

        Self {
            local_cache: Arc::new(DashMap::new()),
            metadata: Arc::new(DashMap::new()),
            chunks: Arc::new(DashMap::new()),
            remote_nodes: Arc::new(RwLock::new(HashMap::new())),
            cache_config,
            shard_manager,
            gc_manager,
        }
    }

    pub async fn put(&self, data: Vec<u8>, owner: String, data_type: String) -> Result<ObjectId> {
        let id = ObjectId::new();
        let size = data.len();

        // 压缩数据
        let (compressed_data, compression_type) = self.compress_data(&data)?;
        
        // 分片数据
        let chunks = self.shard_manager.split_into_chunks(&compressed_data).await?;
        
        // 创建元数据
        let metadata = ObjectMetadata {
            size,
            chunks: chunks.clone(),
            created_at: Instant::now(),
            last_accessed: Arc::new(RwLock::new(Instant::now())),
            access_count: Arc::new(RwLock::new(0)),
            owner,
            data_type,
            compression: compression_type,
            replicas: vec![],
        };

        // 存储分片
        for chunk in chunks {
            self.store_chunk(&chunk).await?;
        }

        // 更新元数据
        self.metadata.insert(id.clone(), metadata);

        // 添加到本地缓存
        self.cache_put(&id, &compressed_data).await?;

        info!("Stored object {} of size {} bytes", id.to_string(), size);
        counter!("object_store.puts", 1);
        gauge!("object_store.total_size", size as f64);

        Ok(id)
    }

    pub async fn get(&self, id: &ObjectId) -> Result<(Vec<u8>, String)> {
        // 更新访问统计
        if let Some(metadata) = self.metadata.get(id) {
            *metadata.last_accessed.write().await = Instant::now();
            *metadata.access_count.write().await += 1;
        }

        // 尝试从缓存获取
        if let Some(entry) = self.local_cache.get(id) {
            info!("Cache hit for object {}", id.to_string());
            counter!("object_store.cache_hits", 1);
            return self.decompress_data(entry.data.to_vec(), self.get_compression_type(id)?);
        }

        // 从分片重建数据
        let data = self.rebuild_from_chunks(id).await?;
        
        // 添加到缓存
        self.cache_put(id, &data).await?;

        counter!("object_store.cache_misses", 1);
        self.decompress_data(data, self.get_compression_type(id)?)
    }

    async fn rebuild_from_chunks(&self, id: &ObjectId) -> Result<Vec<u8>> {
        let metadata = self.metadata.get(id)
            .ok_or_else(|| anyhow!("Object not found"))?;

        let mut data = Vec::with_capacity(metadata.size);
        for chunk in &metadata.chunks {
            let chunk_data = self.get_chunk(&chunk.chunk_id).await?;
            data.extend_from_slice(&chunk_data);
        }

        Ok(data)
    }

    async fn store_chunk(&self, chunk: &ChunkInfo) -> Result<()> {
        // 根据放置策略选择节点
        let nodes = self.select_nodes_for_chunk(chunk).await?;
        
        // 存储到选定的节点
        for node in nodes {
            self.store_chunk_to_node(chunk, &node).await?;
        }

        Ok(())
    }

    async fn select_nodes_for_chunk(&self, chunk: &ChunkInfo) -> Result<Vec<NodeInfo>> {
        let nodes = self.remote_nodes.read().await;
        let mut selected = Vec::new();

        match self.shard_manager.placement_strategy {
            PlacementStrategy::CapacityBased => {
                // 选择可用空间最大的节点
                let mut nodes: Vec<_> = nodes.values().cloned().collect();
                nodes.sort_by_key(|n| n.available_space);
                selected.extend(nodes.iter().take(self.shard_manager.replica_count).cloned());
            }
            PlacementStrategy::LoadBased => {
                // TODO: 实现基于负载的节点选择
            }
            PlacementStrategy::Random => {
                // 随机选择节点
                use rand::seq::IteratorRandom;
                let mut rng = rand::thread_rng();
                selected.extend(
                    nodes.values()
                        .cloned()
                        .choose_multiple(&mut rng, self.shard_manager.replica_count)
                );
            }
        }

        Ok(selected)
    }

    async fn store_chunk_to_node(&self, chunk: &ChunkInfo, node: &NodeInfo) -> Result<()> {
        // TODO: 实现实际的节点间数据传输
        Ok(())
    }

    async fn get_chunk(&self, chunk_id: &str) -> Result<Vec<u8>> {
        if let Some(chunk) = self.chunks.get(chunk_id) {
            return Ok(chunk.to_vec());
        }

        // TODO: 从远程节点获取分片
        Err(anyhow!("Chunk not found"))
    }

    fn compress_data(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionType)> {
        if data.len() < 1024 {  // 小于1KB的数据不压缩
            return Ok((data.to_vec(), CompressionType::None));
        }

        Ok((compress_prepend_size(data), CompressionType::Lz4))
    }

    fn decompress_data(&self, data: Vec<u8>, compression: CompressionType) -> Result<(Vec<u8>, String)> {
        let decompressed = match compression {
            CompressionType::None => data,
            CompressionType::Lz4 => decompress_size_prepended(&data)?,
            CompressionType::Snappy => {
                // TODO: 实现Snappy解压缩
                data
            }
        };

        Ok((decompressed, "application/octet-stream".to_string()))
    }

    fn get_compression_type(&self, id: &ObjectId) -> Result<CompressionType> {
        self.metadata.get(id)
            .map(|m| m.compression.clone())
            .ok_or_else(|| anyhow!("Object not found"))
    }

    async fn cache_put(&self, id: &ObjectId, data: &[u8]) -> Result<()> {
        // 检查缓存大小
        self.evict_if_needed(data.len()).await?;

        // 添加到缓存
        self.local_cache.insert(id.clone(), CacheEntry {
            data: Bytes::from(data.to_vec()),
            inserted_at: Instant::now(),
            last_accessed: Instant::now(),
            size: data.len(),
        });

        Ok(())
    }

    async fn evict_if_needed(&self, required_size: usize) -> Result<()> {
        let mut current_size: usize = self.local_cache.iter().map(|e| e.size).sum();
        
        if current_size + required_size <= self.cache_config.max_size {
            return Ok(());
        }

        // 根据策略选择要驱逐的对象
        let mut entries: Vec<_> = self.local_cache.iter().collect();
        match self.cache_config.eviction_policy {
            EvictionPolicy::LRU => {
                entries.sort_by_key(|e| e.last_accessed);
            }
            EvictionPolicy::LFU => {
                // TODO: 实现LFU策略
            }
            EvictionPolicy::FIFO => {
                entries.sort_by_key(|e| e.inserted_at);
            }
        }

        // 驱逐对象直到有足够空间
        for entry in entries {
            if current_size + required_size <= self.cache_config.max_size {
                break;
            }
            current_size -= entry.size;
            self.local_cache.remove(&entry.key());
        }

        Ok(())
    }

    pub async fn start_gc(&self) {
        let gc_manager = &self.gc_manager;
        let metadata = self.metadata.clone();
        let chunks = self.chunks.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(gc_manager.gc_interval).await;

                let mut last_gc = gc_manager.last_gc.write().await;
                if last_gc.elapsed() < gc_manager.gc_interval {
                    continue;
                }

                info!("Starting garbage collection");
                let deleted = gc_manager.deleted_objects.read().await;
                
                // 删除已标记的对象
                for id in deleted.iter() {
                    if let Some(meta) = metadata.get(id) {
                        // 删除相关的分片
                        for chunk in &meta.chunks {
                            chunks.remove(&chunk.chunk_id);
                        }
                        metadata.remove(id);
                    }
                }

                // 更新最后GC时间
                *last_gc = Instant::now();
                info!("Garbage collection completed");
            }
        });
    }

    pub async fn delete(&self, id: &ObjectId) -> Result<()> {
        // 标记对象为已删除
        self.gc_manager.deleted_objects.write().await.insert(id.clone());
        
        // 从缓存中移除
        self.local_cache.remove(id);

        info!("Marked object {} for deletion", id.to_string());
        counter!("object_store.deletes", 1);
        Ok(())
    }

    pub async fn exists(&self, id: &ObjectId) -> bool {
        self.metadata.contains_key(id)
    }
} 