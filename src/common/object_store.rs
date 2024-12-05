use anyhow::{Result, anyhow};
use uuid::Uuid;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Reverse;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectId(Uuid);

impl ObjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    size: usize,
    created_at: Instant,
    last_accessed: Arc<RwLock<Instant>>,
    access_count: Arc<RwLock<usize>>,
    owner: String,
    data_type: String,
}

pub struct ObjectStore {
    local_store: Arc<RwLock<HashMap<ObjectId, Vec<u8>>>>,
    remote_refs: Arc<RwLock<HashMap<ObjectId, Vec<String>>>>,
    pinned_objects: Arc<RwLock<HashSet<ObjectId>>>,
    metadata: Arc<RwLock<HashMap<ObjectId, ObjectMetadata>>>,
    eviction_config: EvictionConfig,
}

#[derive(Debug, Clone)]
pub struct EvictionConfig {
    max_memory: usize,
    ttl: Duration,
    eviction_policy: EvictionPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvictionPolicy {
    LRU,
    LFU,
    FIFO,
}

impl ObjectStore {
    pub fn new() -> Self {
        Self {
            local_store: Arc::new(RwLock::new(HashMap::new())),
            remote_refs: Arc::new(RwLock::new(HashMap::new())),
            pinned_objects: Arc::new(RwLock::new(HashSet::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            eviction_config: EvictionConfig {
                max_memory: 1024 * 1024 * 1024, // 1GB
                ttl: Duration::from_secs(3600),  // 1 hour
                eviction_policy: EvictionPolicy::LRU,
            },
        }
    }

    pub async fn put(&self, data: Vec<u8>, owner: String, data_type: String) -> Result<ObjectId> {
        let id = ObjectId::new();
        let size = data.len();

        // 检查是否需要驱逐
        if self.get_total_size().await + size > self.eviction_config.max_memory {
            self.evict(size).await?;
        }

        // 创建元数据
        let metadata = ObjectMetadata {
            size,
            created_at: Instant::now(),
            last_accessed: Arc::new(RwLock::new(Instant::now())),
            access_count: Arc::new(RwLock::new(0)),
            owner,
            data_type,
        };

        // 存储对象和元数据
        {
            let mut store = self.local_store.write().await;
            let mut meta = self.metadata.write().await;
            store.insert(id.clone(), data);
            meta.insert(id.clone(), metadata);
        }

        info!("Stored object {} of size {} bytes", id.to_string(), size);
        Ok(id)
    }

    pub async fn get(&self, id: &ObjectId) -> Result<(Vec<u8>, String)> {
        // 首先检查本地存储
        if let Some(data) = self.local_store.read().await.get(id) {
            if let Some(metadata) = self.metadata.read().await.get(id) {
                // 更新访问信息
                *metadata.last_accessed.write().await = Instant::now();
                *metadata.access_count.write().await += 1;
                
                return Ok((data.clone(), metadata.data_type.clone()));
            }
        }

        // 检查远程引用
        if let Some(locations) = self.remote_refs.read().await.get(id) {
            // 尝试从每个远程位置获取数据
            for location in locations {
                if let Ok(data) = self.fetch_from_remote(id, location).await {
                    return Ok(data);
                }
            }
        }

        Err(anyhow!("Object not found"))
    }

    pub async fn pin(&self, id: &ObjectId) -> Result<()> {
        let mut pinned = self.pinned_objects.write().await;
        pinned.insert(id.clone());
        Ok(())
    }

    pub async fn unpin(&self, id: &ObjectId) -> Result<()> {
        let mut pinned = self.pinned_objects.write().await;
        pinned.remove(id);
        Ok(())
    }

    pub async fn delete(&self, id: &ObjectId) -> Result<()> {
        let mut store = self.local_store.write().await;
        let mut meta = self.metadata.write().await;
        let mut remote = self.remote_refs.write().await;
        
        store.remove(id);
        meta.remove(id);
        remote.remove(id);
        
        Ok(())
    }

    async fn get_total_size(&self) -> usize {
        self.metadata.read().await
            .values()
            .map(|m| m.size)
            .sum()
    }

    async fn evict(&self, required_size: usize) -> Result<()> {
        let mut evicted_size = 0;
        let pinned = self.pinned_objects.read().await;
        let mut store = self.local_store.write().await;
        let mut meta = self.metadata.write().await;

        // 获取可以驱逐的对象
        let mut candidates: Vec<_> = meta.iter()
            .filter(|(id, _)| !pinned.contains(id))
            .collect();

        // 根据策略对候选对象排序
        match self.eviction_config.eviction_policy {
            EvictionPolicy::LRU => {
                candidates.sort_by_key(|(_, m)| Reverse(*m.last_accessed.blocking_read()));
            }
            EvictionPolicy::LFU => {
                candidates.sort_by_key(|(_, m)| *m.access_count.blocking_read());
            }
            EvictionPolicy::FIFO => {
                candidates.sort_by_key(|(_, m)| m.created_at);
            }
        }

        // 驱逐对象直到满足空间要求
        for (id, metadata) in candidates {
            if evicted_size >= required_size {
                break;
            }

            store.remove(id);
            meta.remove(id);
            evicted_size += metadata.size;

            info!("Evicted object {} of size {} bytes", id.to_string(), metadata.size);
        }

        if evicted_size < required_size {
            error!("Failed to evict enough space: needed {}, evicted {}", required_size, evicted_size);
            return Err(anyhow!("Could not free enough memory"));
        }

        Ok(())
    }

    async fn fetch_from_remote(&self, id: &ObjectId, location: &str) -> Result<(Vec<u8>, String)> {
        // TODO: 实现实际的远程获取逻辑
        warn!("Remote fetch not implemented for location: {}", location);
        Err(anyhow!("Remote fetch not implemented"))
    }
} 