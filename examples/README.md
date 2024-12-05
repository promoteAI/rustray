# RustRay 示例程序

本目录包含了使用 RustRay 分布式计算框架的示例程序。

## 示例列表

### 1. 矩阵计算 (`matrix/`)
演示如何使用 RustRay 进行大规模矩阵运算：
- `matrix_multiply.rs`: 实现了分布式矩阵乘法
- 将大矩阵分块，分配给多个工作节点并行计算
- 自动合并子任务结果

运行示例：
```bash
cargo run --example matrix_multiply
```

### 2. 素数查找 (`prime/`)
展示如何使用 RustRay 进行大规模数值计算：
- `prime_finder.rs`: 在指定范围内查找素数
- 将数值范围分段，并行处理
- 自动合并所有找到的素数

运行示例：
```bash
cargo run --example prime_finder
```

### 3. 图像处理 (`image_processing/`)
展示如何使用 RustRay 进行并行图像处理：
- `image_filter.rs`: 实现分布式图像滤镜处理
- 将图像分块处理
- 支持多种滤镜效果
- 自动合并处理结果

运行示例：
```bash
cargo run --example image_filter
```

## 使用说明

1. 确保已经启动了 RustRay 的头节点和至少一个工作节点
2. 所有示例默认连接到本地头节点（127.0.0.1:8000）
3. 可以通过环境变量修改连接设置：
   ```bash
   export RUSTRAY_HEAD_HOST=192.168.1.100
   export RUSTRAY_HEAD_PORT=9000
   ```

## 性能优化建议

1. 任务粒度
   - 矩阵计算：建议每个子任务处理 250x250 到 500x500 的块
   - 素数查找：建议每个子任务处理 100万到 1000万个数
   - 图像处理：建议按照图像高度的 1/4 到 1/8 进行分块

2. 资源配置
   - 根据任务特点设置合适的优先级
   - 对计算密集型任务，建议限制每个节点的并发任务数

3. 错误处理
   - 所有示例都包含了基本的错误处理
   - 生产环境中建议添加更多的错误恢复机制

## 扩展示例

你可以基于这些示例开发自己的分布式计算应用：

1. 添加新的计算任务：
```rust
let task = Task {
    name: "your_task_name".to_string(),
    args: vec![/* your args */],
    priority: 0,
};
```

2. 实现任务处理函数：
```rust
#[async_trait]
impl TaskExecutor for YourTask {
    async fn execute(&self, args: Vec<String>) -> Result<Vec<u8>> {
        // 实现你的计算逻辑
    }
}
``` 