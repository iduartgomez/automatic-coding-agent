# Container Orchestration Testing Guide

This guide shows you how to test the container orchestration feature locally.

## Prerequisites

### 1. Install Docker or Podman

**macOS**:
```bash
# Docker Desktop (recommended)
brew install --cask docker

# Or Podman
brew install podman
podman machine init
podman machine start
```

**Linux**:
```bash
# Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER  # Add yourself to docker group
newgrp docker  # Refresh group membership

# Or Podman
sudo apt-get install podman  # Ubuntu/Debian
sudo dnf install podman      # Fedora
```

**Windows**:
- Download [Docker Desktop](https://www.docker.com/products/docker-desktop)
- Or [Podman Desktop](https://podman-desktop.io/)

### 2. Verify Installation

```bash
# Check Docker
docker info
docker run hello-world

# Or Podman
podman info
podman run hello-world
```

## Running the Tests

### Quick Test (Fast Tests Only - 2 minutes)

```bash
# Run all fast container tests (skips slow image builds)
cargo test --test container_orchestration --features containers -- --skip slow

# Expected output: 10 passed; 0 failed
```

### Full Test Suite (20-30 minutes)

```bash
# Run ALL container tests including slow image builds
cargo test --test container_orchestration --features containers

# Expected output: 13 passed; 0 failed
```

### Test Individual Features

```bash
# Test container connection only
cargo test test_container_client_connection --features containers

# Test command execution
cargo test test_exec_command_in_container --features containers

# Test bind mounts
cargo test test_container_with_bind_mounts --features containers

# Test resource limits
cargo test test_container_with_resource_limits --features containers
```

### Run with Output

```bash
# See detailed test output
cargo test --test container_orchestration --features containers -- --nocapture --skip slow
```

## Manual Testing

### 1. Basic Container Operations

Create a file `test_container.rs`:

```rust
use aca::container::{ContainerOrchestrator, ContainerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to Docker/Podman
    let orchestrator = ContainerOrchestrator::new().await?;
    println!("✓ Connected to container runtime");

    // Create a simple container
    let config = ContainerConfig::builder()
        .image("alpine:latest")
        .cmd(vec!["sleep", "60"])
        .build()?;

    let container_id = orchestrator
        .create_container(&config, Some("test-container"))
        .await?;
    println!("✓ Created container: {}", container_id);

    // Start the container
    orchestrator.start_container(&container_id).await?;
    println!("✓ Started container");

    // Execute a command
    let output = orchestrator
        .exec(&container_id, vec!["echo", "Hello from container!"])
        .await?;
    println!("✓ Command output: {}", output.stdout);

    // Cleanup
    orchestrator.stop_and_remove(&container_id).await?;
    println!("✓ Cleaned up container");

    Ok(())
}
```

Run it:
```bash
cargo run --example test_container --features containers
```

### 2. Test with Bind Mounts

```rust
use aca::container::{ContainerOrchestrator, ContainerConfig};
use std::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let orchestrator = ContainerOrchestrator::new().await?;

    // Create a test file on host
    fs::write("/tmp/test-input.txt", "Hello from host!")?;

    let config = ContainerConfig::builder()
        .image("alpine:latest")
        .cmd(vec!["sleep", "60"])
        .bind("/tmp:/host:rw")
        .build()?;

    let container_id = orchestrator.create_container(&config, None).await?;
    orchestrator.start_container(&container_id).await?;

    // Read the file from inside the container
    let output = orchestrator
        .exec(&container_id, vec!["cat", "/host/test-input.txt"])
        .await?;

    println!("File content from container: {}", output.stdout);
    assert_eq!(output.stdout.trim(), "Hello from host!");

    // Write a file from container
    orchestrator
        .exec(&container_id, vec!["sh", "-c", "echo 'Hello from container!' > /host/test-output.txt"])
        .await?;

    // Read it on host
    let content = fs::read_to_string("/tmp/test-output.txt")?;
    println!("File content on host: {}", content);

    orchestrator.stop_and_remove(&container_id).await?;
    Ok(())
}
```

### 3. Test Resource Limits

```rust
use aca::container::{ContainerOrchestrator, ContainerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let orchestrator = ContainerOrchestrator::new().await?;

    let config = ContainerConfig::builder()
        .image("alpine:latest")
        .cmd(vec!["sleep", "60"])
        .memory_limit(512 * 1024 * 1024)  // 512 MB
        .cpu_quota(50_000)  // 50% of one CPU
        .build()?;

    let container_id = orchestrator.create_container(&config, None).await?;
    orchestrator.start_container(&container_id).await?;

    // Monitor resource usage
    let stats = orchestrator.get_stats(&container_id).await?;
    println!("Memory usage: {} bytes", stats.memory_usage);
    println!("CPU usage: {}%", stats.cpu_percentage);

    orchestrator.stop_and_remove(&container_id).await?;
    Ok(())
}
```

## Troubleshooting

### Tests Skip Automatically

If you see:
```
Skipping container tests
test result: ok. 0 passed; 0 failed; 13 ignored
```

**Cause**: Docker/Podman not available or `SKIP_CONTAINER_TESTS=1` set

**Fix**:
```bash
# Check Docker is running
docker info

# Unset skip flag
unset SKIP_CONTAINER_TESTS

# Run tests again
cargo test --test container_orchestration --features containers
```

### Permission Denied

**Linux only**: If you get permission errors:
```bash
# Add yourself to docker group
sudo usermod -aG docker $USER
newgrp docker

# Or use sudo (not recommended for tests)
sudo cargo test --test container_orchestration --features containers
```

### Container Name Conflicts

If tests fail with "name already in use":
```bash
# Clean up test containers
docker rm -f $(docker ps -a --filter "name=aca-test" -q)

# Run tests again
cargo test --test container_orchestration --features containers
```

### Out of Disk Space

Image builds require several GB of space:
```bash
# Check space
df -h

# Clean up old images
docker system prune -a

# Clean up build cache
docker builder prune -a
```

### Tests Hang

If tests hang for >5 minutes on container operations:
```bash
# Check Docker daemon status
docker info

# Restart Docker Desktop (macOS/Windows)
# Or restart docker service (Linux)
sudo systemctl restart docker
```

## CI/CD Testing

The container tests are **not run in CI** by default because CI environments don't always have Docker available. Tests automatically skip when Docker/Podman is unavailable.

To enable in CI, ensure:
1. Docker is installed and running
2. Tests have permission to access Docker socket
3. Sufficient disk space for image builds

## Performance Notes

- **Fast tests**: ~2 minutes (uses `alpine:latest` image)
- **Slow tests**: ~20-30 minutes (builds custom images)
- **Disk usage**: ~5-6 GB (includes all test images)
- **Memory**: Tests use resource limits (512 MB per container)

## Next Steps

After verifying the tests passe

1. **Read the API docs**: `cargo doc --open --features containers`
2. **Try the examples**: See `examples/container_usage.rs` (if exists)
3. **Read the container README**: `container/README.md`
4. **Check the architecture**: `docs/CONTAINER_IMPLEMENTATION_STATUS.md`
