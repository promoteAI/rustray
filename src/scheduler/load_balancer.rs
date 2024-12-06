use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[allow(dead_code)]
pub struct LoadBalancer {
    worker_loads: Arc<RwLock<HashMap<String, f64>>>,
}

#[allow(dead_code)]
impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            worker_loads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_load(&self, worker_id: &str, load: f64) -> Result<()> {
        self.worker_loads.write().await.insert(worker_id.to_string(), load);
        Ok(())
    }

    pub async fn get_load(&self, worker_id: &str) -> Result<Option<f64>> {
        Ok(self.worker_loads.read().await.get(worker_id).copied())
    }

    pub async fn get_least_loaded_worker(&self) -> Result<Option<String>> {
        let loads = self.worker_loads.read().await;
        Ok(loads
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(id, _)| id.clone()))
    }
} 