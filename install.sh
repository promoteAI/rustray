#!/bin/bash

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 打印带颜色的消息
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查命令是否存在
check_command() {
    if ! command -v $1 &> /dev/null; then
        return 1
    fi
    return 0
}

# 检查并安装依赖
install_dependencies() {
    log_info "Checking and installing dependencies..."
    
    # 检查操作系统
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Ubuntu/Debian
        if check_command apt-get; then
            sudo apt-get update
            sudo apt-get install -y \
                build-essential \
                protobuf-compiler \
                cmake \
                pkg-config \
                libssl-dev
        # RHEL/CentOS
        elif check_command yum; then
            sudo yum update
            sudo yum groupinstall -y "Development Tools"
            sudo yum install -y \
                protobuf-compiler \
                cmake \
                openssl-devel
        else
            log_error "Unsupported Linux distribution"
            exit 1
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if ! check_command brew; then
            log_error "Homebrew is required. Please install it first."
            exit 1
        fi
        brew install protobuf cmake pkg-config openssl
    else
        log_error "Unsupported operating system"
        exit 1
    fi
}

# 安装 Rust
install_rust() {
    if ! check_command rustc; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    else
        log_info "Rust is already installed"
    fi
}

# 构建项目
build_project() {
    log_info "Building RustRay..."
    cargo build --release
    if [ $? -ne 0 ]; then
        log_error "Build failed"
        exit 1
    fi
}

# 创建配置文件
create_config() {
    log_info "Creating configuration..."
    
    # 创建配置目录
    mkdir -p ~/.rustray/config
    
    # 生成配置文件
    cat > ~/.rustray/config/config.env << EOF
# RustRay Configuration
RUST_LOG=info
RUSTRAY_AUTH_KEY=$(openssl rand -base64 32)
RUSTRAY_MAX_WORKERS=1000
RUSTRAY_HEARTBEAT_TIMEOUT=30
EOF

    # 设置权限
    chmod 600 ~/.rustray/config/config.env
}

# 主安装流程
main() {
    log_info "Starting RustRay installation..."
    
    # 安装依赖
    install_dependencies
    
    # 安装 Rust
    install_rust
    
    # 构建项目
    build_project
    
    # 创建配置
    create_config
    
    log_info "Installation completed successfully!"
}

# 执行主安装流程
main 