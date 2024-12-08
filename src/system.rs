use std::time::Duration;
use systemstat::{Platform, System};
use crate::models::{SystemStatus, CpuStatus, MemoryStatus, DiskStatus, NetworkStatus};
use crate::error::AppError;

pub struct SystemMonitor {
    sys: System,
    last_network_stats: Option<(u64, u64)>, // (rx, tx)
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            sys: System::new(),
            last_network_stats: None,
        }
    }
    
    pub fn get_status(&mut self) -> Result<SystemStatus, AppError> {
        Ok(SystemStatus {
            cpu: self.get_cpu_status()?,
            memory: self.get_memory_status()?,
            disk: self.get_disk_status()?,
            network: self.get_network_status()?,
        })
    }
    
    fn get_cpu_status(&self) -> Result<CpuStatus, AppError> {
        let cpu = self.sys.cpu_load_aggregate()
            .map_err(|e| AppError::System(e.to_string()))?;
            
        // 等待一秒以获取CPU使用率
        std::thread::sleep(Duration::from_secs(1));
        let cpu = cpu.done()
            .map_err(|e| AppError::System(e.to_string()))?;
            
        Ok(CpuStatus {
            usage: ((cpu.user + cpu.system) * 100.0) as f32,
            cores: num_cpus::get() as u32,
            load: format!("{:.2}", cpu.user + cpu.system),
        })
    }
    
    fn get_memory_status(&self) -> Result<MemoryStatus, AppError> {
        let memory = self.sys.memory()
            .map_err(|e| AppError::System(e.to_string()))?;
            
        let total = memory.total.as_u64();
        let used = total - memory.free.as_u64();
        let usage = (used as f32 / total as f32) * 100.0;
        
        Ok(MemoryStatus {
            usage,
            total,
            used,
        })
    }
    
    fn get_disk_status(&self) -> Result<DiskStatus, AppError> {
        let disk = self.sys.mount_at("/")
            .map_err(|e| AppError::System(e.to_string()))?;
            
        let total = disk.total.as_u64();
        let used = total - disk.free.as_u64();
        let usage = (used as f32 / total as f32) * 100.0;
        
        Ok(DiskStatus {
            usage,
            total,
            used,
        })
    }
    
    fn get_network_status(&mut self) -> Result<NetworkStatus, AppError> {
        let networks = self.sys.networks()
            .map_err(|e| AppError::System(e.to_string()))?;
            
        let mut total_rx = 0;
        let mut total_tx = 0;
        
        for (_interface, stats) in networks {
            let stats = stats.map_err(|e| AppError::System(e.to_string()))?;
            total_rx += stats.rx_bytes.as_u64();
            total_tx += stats.tx_bytes.as_u64();
        }
        
        let (upload_speed, download_speed) = if let Some((last_rx, last_tx)) = self.last_network_stats {
            (
                total_tx.saturating_sub(last_tx),
                total_rx.saturating_sub(last_rx),
            )
        } else {
            (0, 0)
        };
        
        self.last_network_stats = Some((total_rx, total_tx));
        
        Ok(NetworkStatus {
            upload_speed,
            download_speed,
            total_traffic: total_rx + total_tx,
            connections: self.get_connection_count()?,
        })
    }
    
    fn get_connection_count(&self) -> Result<u32, AppError> {
        // 这里简化处理,实际应该读取/proc/net/tcp等文件
        Ok(100)
    }
} 