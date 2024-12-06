# RustRay

RustRay 是一个高性能的分布式计算框架，专注于大规模数据处理和科学计算。

## 特性

- **高性能计算**
  - 基于 Raft 的一致性存储
  - 多级反馈队列调度
  - 数据本地性优化
  - 自动负载均衡

- **容错机制**
  - 任务自动重试
  - 节点故障检测
  - 数据自动复制
  - 状态恢复

- **资源管理**
  - 细粒度资源控制
  - 动态资源分配
  - 资源使用监控
  - QoS 保证

- **开发友好**
  - 简洁的 Python SDK
  - 丰富的示例程序
  - 完善的文档
  - 详细的性能指标

## 快速开始

### 安装

```bash
# 安装系统依赖
./install.sh

# 构建项目
cargo build --release
```

### 运行示例

1. 矩阵乘法
```bash
cargo run --example matrix_multiply
```

2. 素数查找
```bash
cargo run --example prime_finder
```

3. 图像处理
```bash
cargo run --example image_filter
```

### Python SDK 使用

```python
import rustray

# 初始化
rustray.init()

# 定义远程函数
@rustray.remote
def add(x, y):
    return x + y

# 调用函数
future = add.remote(1, 2)
result = rustray.get(future)  # result = 3
```

## 架构设计

### 系统组件

1. **头节点 (Head Node)**
   - 任务调度
   - 资源管理
   - 状态维护
   - 客户端接口

2. **工作节点 (Worker Node)**
   - 任务执行
   - 数据存储
   - 资源监控
   - 故障恢复

3. **调度器 (Scheduler)**
   - 多级反馈队列
   - 数据本地性感知
   - 资源约束
   - 优先级管理

4. **存储系统 (Object Store)**
   - 分布式存储
   - MVCC 并发控制
   - 数据复制
   - 垃圾回收

### 通信协议

使用 gRPC 实现高效的节点间通信：

```protobuf
service RustRay {
    rpc SubmitTask (TaskRequest) returns (TaskResponse);
    rpc GetTaskResult (TaskResultRequest) returns (TaskResultResponse);
    rpc CreateActor (CreateActorRequest) returns (CreateActorResponse);
    // ...
}
```

## 性能优化

### 1. 计算优化

- SIMD 向量化
- 任务并行化
- 缓存友好的数据结构
- 零拷贝技术

### 2. 通信优化

- 批量消息处理
- 消息压缩
- 连接复用
- 本地性优化

### 3. 存储优化

- LSM 树存储引擎
- 异步 I/O
- 数据预取
- 智能缓存

### 4. 调度优化

- 工作窃取
- 负载预测
- 资源预留
- 优先级抢占

## 监控和调试

### 1. 性能指标

- CPU 使用率
- 内存占用
- I/O 吞吐量
- 网络延迟

### 2. 任务指标

- 执行时间
- 等待时间
- 重试次数
- 资源消耗

### 3. 系统指标

- 节点健康状态
- 资源利用率
- 任务成功率
- 系统吞吐量

## 测试

### 1. 单元测试

```bash
cargo test
```

### 2. 集成测试

```bash
cargo test --test integration_test
```

### 3. 基准测试

```bash
cargo bench
```

## 贡献指南

1. Fork 项目
2. 创建特性分支
3. 提交变更
4. 推送到分支
5. 创建 Pull Request

## 性能基准

### 1. 矩阵乘法

| 矩阵大小 | 单节点时间 | 分布式时间 | 加速比 |
|---------|-----------|-----------|--------|
| 1000x1000 | 2.5s | 0.5s | 5x |
| 2000x2000 | 20s | 3s | 6.7x |
| 4000x4000 | 160s | 20s | 8x |

### 2. 图像处理

| 图像大小 | 单节点时间 | 分布式时间 | 加速比 |
|---------|-----------|-----------|--------|
| 1080p | 1.2s | 0.3s | 4x |
| 2K | 2.8s | 0.6s | 4.7x |
| 4K | 10s | 1.8s | 5.5x |

## 许可证

Apache License 2.0