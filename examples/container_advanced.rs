//! Advanced container features example.
//!
//! Demonstrates:
//! - Bind mounts for file sharing
//! - Environment variables
//! - Resource limits
//! - Working directories
//!
//! Run with: cargo run --example container_advanced --features containers

use aca::container::{ContainerConfig, ContainerOrchestrator, ExecConfig};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Advanced Container Features Example\n");

    let orchestrator = ContainerOrchestrator::new().await?;

    // Create temporary directory for testing
    let temp_dir = TempDir::new()?;
    let host_path = temp_dir.path().to_str().unwrap();

    // Write a test file
    fs::write(
        temp_dir.path().join("input.txt"),
        "Hello from the host machine!",
    )?;

    println!("1. Configuring container with advanced options...");
    let config = ContainerConfig::builder()
        .image("alpine:latest")
        .cmd(vec!["sleep", "300"])
        // Bind mount: share directory with container
        .bind(format!("{}:/workspace:rw", host_path))
        // Environment variables
        .env("APP_NAME", "ACA Container Example")
        .env("LOG_LEVEL", "debug")
        // Resource limits
        .memory_limit(512 * 1024 * 1024) // 512 MB
        .cpu_quota(50_000) // 50% of one CPU core
        // Working directory
        .working_dir("/workspace")
        .build()?;

    println!("   ✓ Container configured with:");
    println!("     - Bind mount: {} -> /workspace", host_path);
    println!("     - Memory limit: 512 MB");
    println!("     - CPU limit: 50%\n");

    // Create and start container
    let container_id = orchestrator.create_container(&config, None).await?;
    orchestrator.start_container(&container_id).await?;
    println!("2. Container started\n");

    // Test 1: Read file from bind mount
    println!("3. Testing bind mount (read from host)...");
    let output = orchestrator
        .exec(&container_id, vec!["cat", "/workspace/input.txt"])
        .await?;
    println!("   Content: {}\n", output.stdout.trim());

    // Test 2: Write file from container
    println!("4. Testing bind mount (write from container)...");
    orchestrator
        .exec(
            &container_id,
            vec!["sh", "-c", "echo 'Written by container' > /workspace/output.txt"],
        )
        .await?;

    let content = fs::read_to_string(temp_dir.path().join("output.txt"))?;
    println!("   Host received: {}\n", content.trim());

    // Test 3: Check environment variables
    println!("5. Testing environment variables...");
    let output = orchestrator
        .exec(&container_id, vec!["env"])
        .await?;

    for line in output.stdout.lines() {
        if line.starts_with("APP_NAME=") || line.starts_with("LOG_LEVEL=") {
            println!("   {}", line);
        }
    }
    println!();

    // Test 4: Custom working directory in exec
    println!("6. Testing custom working directory...");

    // Create a directory
    orchestrator
        .exec(&container_id, vec!["mkdir", "-p", "/tmp/test"])
        .await?;

    // Execute with custom working dir
    let exec_config = ExecConfig::builder()
        .cmd(vec!["pwd"])
        .working_dir("/tmp/test")
        .build();

    let output = orchestrator
        .exec_with_config(&container_id, &exec_config)
        .await?;
    println!("   Working directory: {}\n", output.stdout.trim());

    // Cleanup
    println!("7. Cleaning up...");
    orchestrator.stop_and_remove(&container_id).await?;
    println!("   ✓ Container removed\n");

    println!("✅ Advanced example complete!");

    Ok(())
}
