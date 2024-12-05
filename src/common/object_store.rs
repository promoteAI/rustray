use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::Result;
use dashmap::DashMap;

/// 对象ID
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ObjectId(Uuid);

impl ObjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// 对象引用
#[derive(Debug, Clone)]
pub struct ObjectRef {
    pub id: ObjectId,
    pub owner: String,  // 节点ID
    pub size: usize,
}

/// 对象存储
pub struct ObjectStore {
    // 本地存储
    local_store: DashMap<ObjectId, Vec<u8>>,
    // 对象引用表
    references: DashMap<ObjectId, ObjectRef>,
    // 对象位置表
    locations: DashMap<ObjectId, Vec<String>>,
    // 内存限制（字节）
    memory_limit: usize,
}

impl ObjectStore {
    pub fn new(memory_limit: usize) -> Self {
        Self {
            local_store: DashMap::new(),
            references: DashMap::new(),
            locations: DashMap::new(),
            memory_limit,
        }
    }

    /// 存储对象
    pub async fn put(&self, data: Vec<u8>) -> Result<ObjectRef> {
        let id = ObjectId::new();
        let size = data.len();
        
        // 检查内存限制
        if self.get_total_size() + size > self.memory_limit {
            self.evict(size)?;
        }
        
        // 存储数据
        self.local_store.insert(id.clone(), data);
        
        // 创建对象引用
        let obj_ref = ObjectRef {
            id: id.clone(),
            owner: "local".to_string(),
            size,
        };
        
        // 更新引用表和位置表
        self.references.insert(id.clone(), obj_ref.clone());
        self.locations.insert(id, vec!["local".to_string()]);
        
        Ok(obj_ref)
    }

    /// 获取对象
    pub async fn get(&self, id: &ObjectId) -> Result<Option<Vec<u8>>> {
        // 先查本地
        if let Some(data) = self.local_store.get(id) {
            return Ok(Some(data.clone()));
        }
        
        // TODO: 如果本地没有，从其他节点获取
        Ok(None)
    }

    /// 驱逐对象以释放空间
    fn evict(&self, required_size: usize) -> Result<()> {
        let mut evicted_size = 0;
        let mut to_evict = Vec::new();
        
        // 使用简单的 LRU 策略
        for entry in self.local_store.iter() {
            if evicted_size >= required_size {
                break;
            }
            to_evict.push(entry.key().clone());
            evicted_size += entry.value().len();
        }
        
        // 删除选中的对象
        for id in to_evict {
            self.local_store.remove(&id);
            // 更新位置表
            if let Some(mut locations) = self.locations.get_mut(&id) {
                locations.retain(|loc| loc != "local");
            }
        }
        
        Ok(())
    }

    /// 获取当前存储的总大小
    fn get_total_size(&self) -> usize {
        self.local_store.iter().map(|entry| entry.value().len()).sum()
    }

    /// 注册远程对象
    pub async fn register_remote_object(&self, obj_ref: ObjectRef, node_id: String) {
        self.references.insert(obj_ref.id.clone(), obj_ref);
        self.locations.entry(obj_ref.id).or_default().push(node_id);
    }
} 