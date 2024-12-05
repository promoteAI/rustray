use tokio::time::{self, Duration};
use crate::error::Result;
use crate::grpc::rustray::head_service_client::HeadServiceClient;
use crate::grpc::rustray::{RegisterWorkerRequest, HeartbeatRequest, WorkerStatus};
use tonic::transport::Channel;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct ConnectionManager {
    head_addr: String,
    worker_id: String,
    retry_interval: Duration,
    max_retries: u32,
    is_running: Arc<AtomicBool>,
    stats_provider: Arc<dyn WorkerStatsProvider + Send + Sync>,
}

#[async_trait::async_trait]
pub trait WorkerStatsProvider {
    async fn get_current_stats(&self) -> WorkerStatus;
}

impl ConnectionManager {
    pub fn new(
        head_addr: String,
        worker_id: String,
        stats_provider: Arc<dyn WorkerStatsProvider + Send + Sync>,
    ) -> Self {
        Self {
            head_addr,
            worker_id,
            retry_interval: Duration::from_secs(5),
            max_retries: 10,
            is_running: Arc::new(AtomicBool::new(true)),
            stats_provider,
        }
    }

    pub fn shutdown(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub async fn maintain_connection(&self) -> Result<()> {
        let mut retries = 0;
        let mut backoff = ExponentialBackoff::new();

        while self.is_running.load(Ordering::SeqCst) {
            match self.connect().await {
                Ok(mut client) => {
                    info!("Connected to head node");
                    retries = 0;
                    backoff.reset();

                    // Start heartbeat loop
                    while self.is_running.load(Ordering::SeqCst) {
                        match self.send_heartbeat(&mut client).await {
                            Ok(_) => {
                                debug!("Heartbeat sent successfully");
                                tokio::time::sleep(self.retry_interval).await;
                            }
                            Err(e) => {
                                warn!("Heartbeat failed: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    retries += 1;
                    if retries >= self.max_retries {
                        error!("Max retries reached, giving up: {}", e);
                        return Err(e);
                    }
                    let delay = backoff.next_backoff();
                    warn!("Connection failed, retrying in {:?}... Error: {}", delay, e);
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Ok(())
    }

    async fn connect(&self) -> Result<HeadServiceClient<Channel>> {
        let client = HeadServiceClient::connect(format!("http://{}", self.head_addr)).await?;
        
        // Register with head node
        let request = tonic::Request::new(RegisterWorkerRequest {
            worker_id: self.worker_id.clone(),
            address: self.head_addr.clone(),
            port: 0,
        });

        client.clone().register_worker(request).await?;
        Ok(client)
    }

    async fn send_heartbeat(&self, client: &mut HeadServiceClient<Channel>) -> Result<()> {
        let status = self.stats_provider.get_current_stats().await;
        let request = tonic::Request::new(HeartbeatRequest {
            worker_id: self.worker_id.clone(),
            status: Some(status),
        });

        client.heartbeat(request).await?;
        Ok(())
    }
}

struct ExponentialBackoff {
    current: Duration,
    max: Duration,
    multiplier: f64,
}

impl ExponentialBackoff {
    fn new() -> Self {
        Self {
            current: Duration::from_secs(1),
            max: Duration::from_secs(60),
            multiplier: 2.0,
        }
    }

    fn next_backoff(&mut self) -> Duration {
        let current = self.current;
        self.current = std::cmp::min(
            self.current.mul_f64(self.multiplier),
            self.max,
        );
        current
    }

    fn reset(&mut self) {
        self.current = Duration::from_secs(1);
    }
} 