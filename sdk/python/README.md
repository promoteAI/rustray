# RustRay Python SDK

RustRay Python SDK provides a simple and efficient way to use RustRay distributed computing framework in Python.

## Installation

```bash
pip install rustray
```

## Quick Start

```python
import rustray

# Initialize RustRay
rustray.init()

# Define a remote function
@rustray.remote
def add(x, y):
    return x + y

# Call the function
future = add.remote(1, 2)
result = rustray.get(future)  # result = 3

# Define an actor
@rustray.actor
class Counter:
    def __init__(self):
        self.value = 0
    
    def increment(self):
        self.value += 1
        return self.value

# Create an actor instance
counter = Counter.remote()
future = counter.increment.remote()
value = rustray.get(future)  # value = 1
```

## Features

- Remote function execution
- Distributed actors
- Object store for efficient data sharing
- Dynamic task scheduling
- Automatic resource management
- Fault tolerance

## Examples

### Matrix Multiplication

```python
import rustray
import numpy as np

@rustray.remote
def multiply(a, b):
    return np.dot(a, b)

# Initialize matrices
a = np.random.rand(1000, 1000)
b = np.random.rand(1000, 1000)

# Compute in parallel
result = rustray.get(multiply.remote(a, b))
```

### Distributed Data Processing

```python
import rustray

@rustray.remote
class DataProcessor:
    def process_chunk(self, data):
        # Process data chunk
        return processed_data

# Create processor instances
processors = [DataProcessor.remote() for _ in range(4)]

# Process data in parallel
futures = [p.process_chunk.remote(chunk) for p, chunk in zip(processors, data_chunks)]
results = rustray.get(futures)
```

## Documentation

For detailed documentation, visit: https://docs.rustray.org

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

Apache License 2.0 