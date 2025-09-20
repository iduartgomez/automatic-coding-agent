//! Demonstration of setup commands and error handling strategies
//!
//! This example shows how to configure and use setup commands with various
//! error handling strategies including skip, retry, and backup commands.

use automatic_coding_agent::{AgentConfig, AgentSystem};
use automatic_coding_agent::task::{SetupCommand, ErrorHandler, OutputCondition};
use chrono::Duration;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Setup Commands Demo");
    println!("======================");

    // Create setup commands demonstrating different scenarios
    let setup_commands = vec![
        // 1. Simple command that should succeed
        SetupCommand::new("check_rust", "rustc")
            .with_args(vec!["--version".to_string()]),

        // 2. Command with working directory
        SetupCommand::new("list_files", "ls")
            .with_args(vec!["-la".to_string()])
            .with_working_dir(PathBuf::from(".")),

        // 3. Optional command that might fail (skip on failure)
        SetupCommand::new("optional_check", "which")
            .with_args(vec!["nonexistent-command".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::skip("skip_missing_tool")),

        // 4. Command with retry strategy
        SetupCommand::new("network_check", "ping")
            .with_args(vec!["-c".to_string(), "1".to_string(), "8.8.8.8".to_string()])
            .with_timeout(Duration::seconds(5))
            .with_error_handler(ErrorHandler::retry(
                "retry_ping",
                3,
                Duration::seconds(1)
            )),

        // 5. Command with backup strategy based on stderr analysis
        SetupCommand::new("check_docker", "docker")
            .with_args(vec!["--version".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "docker_backup",
                OutputCondition::stderr_contains("command not found"),
                "echo",
                vec!["Docker not available - using alternative approach".to_string()]
            )),

        // 6. Command that creates a temporary file
        SetupCommand::new("create_temp", "touch")
            .with_args(vec!["/tmp/setup_test.txt".to_string()])
            .with_timeout(Duration::seconds(10)),
    ];

    // Configure agent with setup commands
    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("setup_demo"),
        setup_commands,
        ..Default::default()
    };

    println!("📋 Configured {} setup commands", config.setup_commands.len());
    println!();

    // Create agent system (this will execute setup commands)
    println!("🔧 Initializing agent system with setup commands...");
    let start_time = std::time::Instant::now();

    match AgentSystem::new(config).await {
        Ok(_agent) => {
            let duration = start_time.elapsed();
            println!("✅ Agent system initialized successfully in {:?}", duration);
            println!();

            // Check if temporary file was created
            if std::path::Path::new("/tmp/setup_test.txt").exists() {
                println!("✅ Temporary file created successfully");
                // Clean up
                std::fs::remove_file("/tmp/setup_test.txt").ok();
                println!("🧹 Cleaned up temporary file");
            } else {
                println!("❌ Temporary file was not created");
            }
        }
        Err(e) => {
            println!("❌ Agent system initialization failed: {}", e);
            return Err(e);
        }
    }

    println!();
    println!("🎉 Setup commands demo completed successfully!");

    Ok(())
}