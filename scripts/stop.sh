#!/bin/bash

# 停止头节点
if [ -f logs/head.pid ]; then
    echo "Stopping head node..."
    kill $(cat logs/head.pid)
    rm logs/head.pid
fi

# 停止所有工作节点
for pid_file in logs/worker_*.pid; do
    if [ -f "$pid_file" ]; then
        echo "Stopping worker node ($pid_file)..."
        kill $(cat $pid_file)
        rm $pid_file
    fi
done

echo "All nodes stopped" 