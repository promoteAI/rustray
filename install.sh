#!/bin/bash

# 安装系统依赖
echo "[INFO] Installing system dependencies..."
if command -v apt-get > /dev/null; then
    sudo apt-get update
    sudo apt-get install -y build-essential protobuf-compiler cmake pkg-config libssl-dev
elif command -v yum > /dev/null; then
    sudo yum update
    sudo yum groupinstall -y "Development Tools"
    sudo yum install -y protobuf-compiler cmake openssl-devel
else
    echo "[ERROR] Unsupported package manager"
    exit 1
fi

# 安装 Rust
echo "[INFO] Installing Rust..."
if ! command -v rustc > /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# 构建项目
echo "[INFO] Building project..."
cargo build --release

echo "[INFO] Installation completed!" 