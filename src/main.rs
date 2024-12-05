mod common;
mod grpc;
mod scheduler;
mod worker;

use grpc::rustray::rust_ray_server::RustRayServer;
use grpc::RustRayService;

use std::net::SocketAddr;
use tonic::transport::Server;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create service
    let service = RustRayService {};

    // Get address to bind to
    let addr: SocketAddr = "[::1]:50051".parse()?;

    println!("RustRay server listening on {}", addr);

    // Start server
    Server::builder()
        .add_service(RustRayServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
} 