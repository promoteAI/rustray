use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use dashmap::DashMap;
use std::time::{Duration, SystemTime};

/// 对象ID
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ObjectId(Uuid);

impl ObjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// 对象元数据
#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    pub size: usize,
    pub created_at: SystemTime,
    pub owner: String,
    pub ref_count: usize,
    pub location_hints: Vec<String>,  // 存储位置提示
    pub data_type: String,  // 数据类型标识
}

/// 对象存储
pub struct ObjectStore {
    // 本地共享内存存储
    local_store: DashMap<ObjectId, Arc<Bytes>>,
    // 对象元数据
    metadata: DashMap<ObjectId, ObjectMetadata>,
    // 远程对象引用
    remote_refs: DashMap<ObjectId, Vec<String>>,
    // 内存限制
    memory_limit: usize,
    // 驱逐策略配置
    eviction_config: EvictionConfig,
}

/// 驱逐策略配置
#[derive(Debug, Clone)]
pub struct EvictionConfig {
    pub max_age: Duration,
    pub min_ref_count: usize,
    pub target_memory_ratio: f64,
}

impl Default for EvictionConfig {
    fn default() -> Self {
        Self {
            max_age: Duration::from_secs(3600),  // 1小时
            min_ref_count: 2,
            target_memory_ratio: 0.8,
        }
    }
}

impl ObjectStore {
    pub fn new() -> Self {
        Self::with_config(EvictionConfig::default())
    }

    pub fn with_config(config: EvictionConfig) -> Self {
        Self {
            local_store: DashMap::new(),
            metadata: DashMap::new(),
            remote_refs: DashMap::new(),
            memory_limit: 1024 * 1024 * 1024,  // 1GB 默认限制
            eviction_config: config,
        }
    }

    /// 存储对象
    pub async fn put(&self, data: Vec<u8>, owner: String, data_type: String) -> Result<ObjectId> {
        let id = ObjectId::new();
        let size = data.len();

        // 检查内存限制
        if self.get_total_size() + size > self.memory_limit {
            self.evict(size).await?;
        }

        // 创建元数据
        let metadata = ObjectMetadata {
            size,
            created_at: SystemTime::now(),
            owner,
            ref_count: 1,
            location_hints: vec!["local".to_string()],
            data_type,
        };

        // 存储数据和元数据
        self.local_store.insert(id.clone(), Arc::new(Bytes::from(data)));
        self.metadata.insert(id.clone(), metadata);

        Ok(id)
    }

    /// 获取对象
    pub async fn get(&self, id: &ObjectId) -> Result<(Vec<u8>, String)> {
        // 先检查本地存储
        if let Some(data) = self.local_store.get(id) {
            if let Some(metadata) = self.metadata.get(id) {
                return Ok((data.to_vec(), metadata.data_type.clone()));
            }
        }

        // 检查远程引用
        if let Some(locations) = self.remote_refs.get(id) {
            // TODO: 实现从远程节点获取数据
            return Err(anyhow!("Remote fetch not implemented"));
        }

        Err(anyhow!("Object not found"))
    }

    /// 删除对象
    pub async fn delete(&self, id: &ObjectId) -> Result<bool> {
        let mut success = false;

        // 删除本地数据
        if self.local_store.remove(id).is_some() {
            success = true;
        }

        // 删除元数据
        self.metadata.remove(id);

        // 删除远程引用
        self.remote_refs.remove(id);

        Ok(success)
    }

    /// 固定对象
    pub async fn pin(&self, id: &ObjectId) -> Result<()> {
        if let Some(mut metadata) = self.metadata.get_mut(id) {
            metadata.ref_count += 1;
            Ok(())
        } else {
            Err(anyhow!("Object not found"))
        }
    }

    /// 释放对象
    pub async fn unpin(&self, id: &ObjectId) -> Result<()> {
        if let Some(mut metadata) = self.metadata.get_mut(id) {
            if metadata.ref_count > 0 {
                metadata.ref_count -= 1;
            }
            Ok(())
        } else {
            Err(anyhow!("Object not found"))
        }
    }

    /// 注册远程对象
    pub async fn register_remote_object(
        &self,
        id: ObjectId,
        metadata: ObjectMetadata,
        locations: Vec<String>,
    ) -> Result<()> {
        self.metadata.insert(id.clone(), metadata);
        self.remote_refs.insert(id, locations);
        Ok(())
    }

    /// 驱逐对象以释放空间
    async fn evict(&self, required_size: usize) -> Result<()> {
        let mut evicted_size = 0;
        let now = SystemTime::now();
        let config = &self.eviction_config;

        // 收集可以驱逐的对象
        let mut candidates: Vec<(ObjectId, ObjectMetadata)> = self.metadata
            .iter()
            .filter(|entry| {
                let metadata = entry.value();
                metadata.ref_count <= config.min_ref_count &&
                metadata.created_at.elapsed().unwrap() > config.max_age
            })
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        // 按年龄排序
        candidates.sort_by_key(|(_, metadata)| metadata.created_at);

        // 驱逐对象直到满足要求
        for (id, metadata) in candidates {
            if evicted_size >= required_size {
                break;
            }

            if self.local_store.remove(&id).is_some() {
                evicted_size += metadata.size;
                self.metadata.remove(&id);
            }
        }

        if evicted_size < required_size {
            return Err(anyhow!("Could not free enough memory"));
        }

        Ok(())
    }

    /// 获取当前存储的总大小
    fn get_total_size(&self) -> usize {
        self.metadata.iter()
            .filter(|entry| entry.value().location_hints.contains(&"local".to_string()))
            .map(|entry| entry.value().size)
            .sum()
    }

    /// 获取对象位置
    pub async fn get_locations(&self, id: &ObjectId) -> Option<Vec<String>> {
        if self.local_store.contains_key(id) {
            Some(vec!["local".to_string()])
        } else {
            self.remote_refs.get(id).map(|refs| refs.to_vec())
        }
    }

    /// 获取对象大小
    pub async fn get_object_size(&self, id: &ObjectId) -> Option<usize> {
        self.metadata.get(id).map(|metadata| metadata.size)
    }
} 