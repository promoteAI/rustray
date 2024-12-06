#!/bin/bash

# 检查命令行参数
if [ $# -lt 1 ]; then
    echo "Usage: $0 [head|worker] [port]"
    exit 1
fi

# 设置默认端口
HEAD_PORT=${2:-8000}
WORKER_PORT=${2:-8001}

# 创建日志目录
mkdir -p logs

case $1 in
    "head")
        echo "Starting head node on port $HEAD_PORT..."
        RUST_LOG=info cargo run --release -- --node-type head --port $HEAD_PORT > logs/head.log 2>&1 &
        echo $! > logs/head.pid
        echo "Head node started (PID: $(cat logs/head.pid))"
        ;;
        
    "worker")
        echo "Starting worker node on port $WORKER_PORT..."
        RUST_LOG=info cargo run --release -- --node-type worker --port $WORKER_PORT --head-addr "127.0.0.1:$HEAD_PORT" > logs/worker_$WORKER_PORT.log 2>&1 &
        echo $! > logs/worker_$WORKER_PORT.pid
        echo "Worker node started (PID: $(cat logs/worker_$WORKER_PORT.pid))"
        ;;
        
    *)
        echo "Invalid node type. Use 'head' or 'worker'"
        exit 1
        ;;
esac 