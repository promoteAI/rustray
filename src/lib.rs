pub mod api;
pub mod common;
pub mod error;
pub mod grpc;
pub mod head;
pub mod metrics;
pub mod scheduler;
pub mod worker;
pub mod cache;

pub mod proto {
    tonic::include_proto!("rustray");
}

pub use error::{Result, RustRayError};
pub use metrics::MetricsCollector;
pub use proto::*;

use std::sync::Arc;
use crate::worker::WorkerNode;

#[derive(Clone)]
pub struct AppState {
    pub metrics: Arc<MetricsCollector>,
    pub worker: Arc<WorkerNode>,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

pub mod cache {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use std::time::{Duration, Instant};

    #[derive(Clone)]
    pub struct CacheEntry<T> {
        pub data: T,
        pub expires_at: Instant,
    }

    pub struct MultiLevelCache<T: Clone + Send + Sync + 'static> {
        l1_cache: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,  // 内存缓存
        l2_cache: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,  // 分布式缓存
    }

    impl<T: Clone + Send + Sync> MultiLevelCache<T> {
        pub fn new() -> Self {
            Self {
                l1_cache: Arc::new(RwLock::new(HashMap::new())),
                l2_cache: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub async fn get(&self, key: &str) -> Option<T> {
            // 先查L1缓存
            if let Some(entry) = self.l1_cache.read().await.get(key) {
                if entry.expires_at > Instant::now() {
                    return Some(entry.data.clone());
                }
            }

            // L1未命中，查L2缓存
            if let Some(entry) = self.l2_cache.read().await.get(key) {
                if entry.expires_at > Instant::now() {
                    // 写入L1缓存
                    let mut l1 = self.l1_cache.write().await;
                    l1.insert(key.to_string(), entry.clone());
                    return Some(entry.data.clone());
                }
            }

            None
        }

        pub async fn set(&self, key: String, value: T, ttl: Duration) {
            let entry = CacheEntry {
                data: value.clone(),
                expires_at: Instant::now() + ttl,
            };

            // 同时写入L1和L2缓存
            self.l1_cache.write().await.insert(key.clone(), entry.clone());
            self.l2_cache.write().await.insert(key, entry);
        }
    }
}