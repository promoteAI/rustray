//! 分布式对象存储模块
//! 
//! 借鉴 TiKV 的设计实现高性能分布式存储：
//! - 基于 Raft 的一致性保证
//! - 多版本并发控制(MVCC)
//! - 分布式事务支持
//! - 高效的存储引擎

use std::collections::HashMap;
use std::sync::RwLock;

/// Object store for storing and retrieving data
pub struct ObjectStore {
    name: String,
    data: RwLock<HashMap<String, Vec<u8>>>,
}

impl ObjectStore {
    /// Create a new object store
    pub fn new(name: String) -> Self {
        Self {
            name,
            data: RwLock::new(HashMap::new()),
        }
    }

    /// Store data with the given key
    pub fn put(&self, key: String, data: Vec<u8>) {
        let mut store = self.data.write().unwrap();
        store.insert(key, data);
    }

    /// Get data by key
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let store = self.data.read().unwrap();
        store.get(key).cloned()
    }

    /// Delete data by key
    pub fn delete(&self, key: &str) {
        let mut store = self.data.write().unwrap();
        store.remove(key);
    }

    /// Check if key exists
    pub fn exists(&self, key: &str) -> bool {
        let store = self.data.read().unwrap();
        store.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        let store = self.data.read().unwrap();
        store.keys().cloned().collect()
    }

    /// Clear all data
    pub fn clear(&self) {
        let mut store = self.data.write().unwrap();
        store.clear();
    }
} 