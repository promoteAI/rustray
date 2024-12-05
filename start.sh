#!/bin/bash

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 加载配置
if [ -f ~/.rustray/config/config.env ]; then
    source ~/.rustray/config/config.env
else
    echo -e "${RED}[ERROR]${NC} Configuration file not found. Please run install.sh first."
    exit 1
fi

# 默认值
DEFAULT_HEAD_PORT=8000
DEFAULT_WORKER_PORT=8001
DEFAULT_HEAD_ADDR="localhost:8000"

# 帮助信息
show_help() {
    echo "Usage:"
    echo "  Start head node:    ./start.sh head [port]"
    echo "  Start worker node:  ./start.sh worker [head-address] [port]"
    echo ""
    echo "Examples:"
    echo "  ./start.sh head 8000"
    echo "  ./start.sh worker localhost:8000 8001"
}

# 启动头节点
start_head() {
    local port=${1:-$DEFAULT_HEAD_PORT}
    echo -e "${GREEN}[INFO]${NC} Starting head node on port $port..."
    RUST_LOG=$RUST_LOG \
    RUSTRAY_AUTH_KEY=$RUSTRAY_AUTH_KEY \
    cargo run --release -- --role head --port $port
}

# 启动工作节点
start_worker() {
    local head_addr=${1:-$DEFAULT_HEAD_ADDR}
    local port=${2:-$DEFAULT_WORKER_PORT}
    echo -e "${GREEN}[INFO]${NC} Starting worker node on port $port, connecting to $head_addr..."
    RUST_LOG=$RUST_LOG \
    RUSTRAY_AUTH_KEY=$RUSTRAY_AUTH_KEY \
    cargo run --release -- --role worker --head-addr $head_addr --port $port
}

# 主函数
main() {
    case "$1" in
        "head")
            start_head $2
            ;;
        "worker")
            start_worker $2 $3
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            echo -e "${RED}[ERROR]${NC} Invalid command"
            show_help
            exit 1
            ;;
    esac
}

# 执行主函数
main "$@" 