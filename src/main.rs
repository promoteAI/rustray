//! 主程序入口

use clap::{Parser, ValueEnum};
use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::{info, warn, error};

// 从 crate 根导入模块
use rustray::{
    grpc::{
        rustray::rust_ray_server::RustRayServer,
        RustRayService,
    },
    head::HeadNode,
    worker::WorkerNode,
    error::Result,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 节点类型
    #[arg(short, long, value_enum)]
    node_type: NodeType,

    /// 监听端口
    #[arg(short, long, default_value_t = 8000)]
    port: u16,

    /// 头节点地址 (仅工作节点需要)
    #[arg(short, long, default_value = "127.0.0.1:8000")]
    head_addr: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum NodeType {
    Head,
    Worker,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化更详细的日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .compact()
        .init();

    // 解析命令行参数
    let args = Args::parse();

    // 构建监听地址
    let addr: SocketAddr = match format!("0.0.0.0:{}", args.port).parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid address: {}", e);
            return Err(anyhow::anyhow!("Invalid address: {}", e).into());
        }
    };

    // 设置 Ctrl+C 处理
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C, shutting down...");
        let _ = shutdown_tx.send(());
    });

    match args.node_type {
        NodeType::Head => {
            // 启动头节点
            info!("Starting head node on {}", addr);
            let head = match HeadNode::new(addr.ip().to_string(), addr.port()) {
                Ok(head) => head,
                Err(e) => {
                    error!("Failed to create head node: {}", e);
                    return Err(e);
                }
            };

            let service = RustRayService::new(head);
            
            let server = Server::builder()
                .add_service(RustRayServer::new(service));

            info!("Head node server starting...");
            
            tokio::select! {
                result = server.serve(addr) => {
                    match result {
                        Ok(_) => info!("Server shutdown gracefully"),
                        Err(e) => error!("Server error: {}", e)
                    }
                }
                _ = shutdown_rx => {
                    info!("Shutdown signal received");
                }
            }
        }
        NodeType::Worker => {
            // 启动工作节点
            info!("Starting worker node on {} (head: {})", addr, args.head_addr);
            let worker = match WorkerNode::new(addr.ip().to_string(), addr.port()) {
                Ok(worker) => worker,
                Err(e) => {
                    error!("Failed to create worker node: {}", e);
                    return Err(e);
                }
            };

            info!("Connecting to head node at {}", args.head_addr);
            match worker.connect_to_head(&args.head_addr).await {
                Ok(_) => info!("Successfully connected to head node"),
                Err(e) => {
                    error!("Failed to connect to head node: {}", e);
                    return Err(e);
                }
            }
            
            // 等待关闭信号
            tokio::select! {
                _ = shutdown_rx => {
                    info!("Worker node shutting down");
                }
            }
        }
    }

    info!("Node shutdown complete");
    Ok(())
} 