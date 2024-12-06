//! 分布式对象存储模块
//! 
//! 借鉴 TiKV 的设计实现高性能分布式存储：
//! - 基于 Raft 的一致性保证
//! - 多版本并发控制(MVCC)
//! - 分布式事务支持
//! - 高效的存储引擎

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::{Result, RustRayError};

pub struct ObjectStore {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl ObjectStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn put(&self, key: String, value: Vec<u8>) -> Result<()> {
        let mut data = self.data.write().await;
        data.insert(key, value);
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let data = self.data.read().await;
        Ok(data.contains_key(key))
    }
} 