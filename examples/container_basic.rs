//! Basic container orchestration example.
//!
//! This example demonstrates the core container operations:
//! - Creating and starting a container
//! - Executing commands
//! - Cleaning up
//!
//! Run with: cargo run --example container_basic --features containers

use aca::container::{ContainerConfig, ContainerOrchestrator};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🐳 Container Orchestration Example\n");

    // Step 1: Connect to Docker/Podman
    println!("1. Connecting to container runtime...");
    let orchestrator = ContainerOrchestrator::new().await?;
    println!("   ✓ Connected successfully\n");

    // Step 2: Configure container
    println!("2. Configuring container...");
    let config = ContainerConfig::builder()
        .image("alpine:latest") // Small Linux image
        .cmd(vec!["sleep", "300"]) // Keep container alive
        .build()?;
    println!("   ✓ Configuration ready\n");

    // Step 3: Create container
    println!("3. Creating container...");
    let container_id = orchestrator
        .create_container(&config, Some("aca-example"))
        .await?;
    println!("   ✓ Created: {}\n", &container_id[..12]);

    // Step 4: Start container
    println!("4. Starting container...");
    orchestrator.start_container(&container_id).await?;
    println!("   ✓ Container running\n");

    // Step 5: Execute commands
    println!("5. Executing commands...");

    // Simple echo
    let output = orchestrator
        .exec(&container_id, vec!["echo", "Hello from container!"])
        .await?;
    println!("   Output: {}", output.stdout.trim());

    // Check Alpine version
    let output = orchestrator
        .exec(&container_id, vec!["cat", "/etc/alpine-release"])
        .await?;
    println!("   Alpine version: {}", output.stdout.trim());

    // List files
    let output = orchestrator
        .exec(&container_id, vec!["ls", "-la", "/"])
        .await?;
    println!("   Root directory:\n{}", output.stdout);

    // Step 6: Cleanup
    println!("6. Cleaning up...");
    orchestrator.stop_and_remove(&container_id).await?;
    println!("   ✓ Container removed\n");

    println!("✅ Example complete!");

    Ok(())
}
