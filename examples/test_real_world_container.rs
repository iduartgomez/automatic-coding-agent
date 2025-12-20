//! Real-world container usage test - simulates aca task execution in isolation
//!
//! This demonstrates how aca WILL use containers once integrated with the CLI:
//! 1. Create isolated container for a coding task
//! 2. Mount workspace directory
//! 3. Execute coding operations (file creation, git operations, etc.)
//! 4. Verify results on host
//! 5. Clean up container
//!
//! Run with: cargo run --example test_real_world_container --features containers

use aca::container::{ContainerConfig, ContainerOrchestrator};
use anyhow::Result;
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Real-World Container Test: Isolated Task Execution\n");
    println!("This simulates how aca will execute tasks in containers\n");

    // Step 1: Setup workspace (simulating user's project)
    println!("1. Setting up workspace...");
    let workspace = TempDir::new()?;
    let workspace_path = workspace.path().to_str().unwrap();

    // Create a simple project structure
    fs::write(
        workspace.path().join("README.md"),
        "# Test Project\n\nA test project for container execution.",
    )?;
    println!("   ✓ Created workspace at: {}\n", workspace_path);

    // Step 2: Create container orchestrator
    println!("2. Connecting to container runtime...");
    let orchestrator = ContainerOrchestrator::new().await?;
    println!("   ✓ Connected successfully\n");

    // Step 3: Configure container (simulating aca's configuration)
    println!("3. Configuring isolated execution environment...");
    let config = ContainerConfig::builder()
        .image("alpine:latest")
        .cmd(vec!["sleep", "300"])
        // Mount workspace for file operations
        .bind(format!("{}:/workspace:rw", workspace_path))
        // Set environment variables (simulating aca context)
        .env("ACA_SESSION_ID", "test-session-123")
        .env("ACA_TASK_ID", "task-456")
        .env("ACA_MODE", "container")
        // Resource limits (prevent runaway tasks)
        .memory_limit(512 * 1024 * 1024) // 512 MB
        .cpu_quota(50_000) // 50% of one CPU
        // Working directory
        .working_dir("/workspace")
        .build()?;
    println!("   ✓ Container configured with:");
    println!("     - Image: alpine:latest");
    println!("     - Workspace: {} -> /workspace", workspace_path);
    println!("     - Memory: 512 MB");
    println!("     - CPU: 50%\n");

    // Step 4: Launch container
    println!("4. Launching isolated container...");
    let container_id = orchestrator
        .create_container(&config, Some("aca-test-task"))
        .await?;
    orchestrator.start_container(&container_id).await?;
    println!("   ✓ Container running: {}\n", &container_id[..12]);

    // Step 5: Execute coding tasks (simulating what aca would do)
    println!("5. Executing coding tasks in isolation...\n");

    // Task 1: Create a new source file
    println!("   Task 1: Creating source file...");
    orchestrator
        .exec(
            &container_id,
            vec![
                "sh",
                "-c",
                "cat > /workspace/main.rs << 'EOF'\nfn main() {\n    println!(\"Hello from isolated container!\");\n}\nEOF",
            ],
        )
        .await?;
    println!("   ✓ Created main.rs");

    // Task 2: List files
    println!("\n   Task 2: Listing workspace files...");
    let output = orchestrator
        .exec(&container_id, vec!["ls", "-lah", "/workspace"])
        .await?;
    println!("   Workspace contents:\n{}", output.stdout);

    // Task 3: Git initialization (common aca task)
    println!("   Task 3: Initializing git repository...");

    // Install git first (alpine doesn't have it by default)
    orchestrator
        .exec(&container_id, vec!["apk", "add", "--no-cache", "git"])
        .await?;

    orchestrator
        .exec(&container_id, vec!["git", "init"])
        .await?;

    orchestrator
        .exec(
            &container_id,
            vec!["git", "config", "user.email", "aca@test.com"],
        )
        .await?;

    orchestrator
        .exec(
            &container_id,
            vec!["git", "config", "user.name", "ACA Test"],
        )
        .await?;

    orchestrator
        .exec(&container_id, vec!["git", "add", "."])
        .await?;

    orchestrator
        .exec(
            &container_id,
            vec!["git", "commit", "-m", "Initial commit from container"],
        )
        .await?;
    println!("   ✓ Initialized git and created commit");

    // Task 4: Create a build script
    println!("\n   Task 4: Creating build script...");
    orchestrator
        .exec(
            &container_id,
            vec![
                "sh",
                "-c",
                "cat > /workspace/build.sh << 'EOF'\n#!/bin/sh\necho \"Building project...\"\necho \"Build complete!\"\nEOF",
            ],
        )
        .await?;

    orchestrator
        .exec(&container_id, vec!["chmod", "+x", "/workspace/build.sh"])
        .await?;

    let output = orchestrator
        .exec(&container_id, vec!["/workspace/build.sh"])
        .await?;
    println!("   ✓ Created and executed build script");
    println!("   Output: {}", output.stdout.trim());

    // Step 6: Verify results on host
    println!("\n6. Verifying results on host machine...");

    let main_rs = fs::read_to_string(workspace.path().join("main.rs"))?;
    println!("   ✓ main.rs exists: {} bytes", main_rs.len());

    let git_dir = workspace.path().join(".git");
    println!("   ✓ .git directory exists: {}", git_dir.exists());

    let build_sh = workspace.path().join("build.sh");
    println!("   ✓ build.sh exists: {}", build_sh.exists());

    println!("\n   📁 Final workspace contents:");
    for entry in fs::read_dir(workspace.path())? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let size = if metadata.is_file() {
            format!("{} bytes", metadata.len())
        } else {
            "directory".to_string()
        };
        println!("     - {} ({})", entry.file_name().to_string_lossy(), size);
    }

    // Step 7: Cleanup
    println!("\n7. Cleaning up...");
    orchestrator.stop_and_remove(&container_id).await?;
    println!("   ✓ Container removed\n");

    println!("✅ Real-world test complete!\n");
    println!("Summary:");
    println!("  • Created isolated execution environment");
    println!("  • Executed multiple coding tasks safely");
    println!("  • Results persisted to host filesystem");
    println!("  • Container cleaned up automatically");
    println!("\nThis demonstrates how aca will use containers for:");
    println!("  - Isolated task execution");
    println!("  - Safe workspace manipulation");
    println!("  - Git operations in sandboxed environment");
    println!("  - Resource-limited execution");

    Ok(())
}
