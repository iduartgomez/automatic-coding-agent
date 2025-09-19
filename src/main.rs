use automatic_coding_agent::{AgentSystem, AgentConfig};
use std::io::{self, Write};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("automatic_coding_agent=info")
        .init();

    info!("Starting Automatic Coding Agent");

    // Create agent config
    let config = AgentConfig::default();

    // Initialize the agent system
    info!("Initializing agent system...");
    let agent = AgentSystem::new(config).await?;

    info!("Agent system initialized successfully!");

    // Simple CLI loop
    loop {
        print!("\n> Enter a task description (or 'quit' to exit): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "quit" || input == "exit" {
            break;
        }

        if input == "status" {
            show_system_status(&agent).await?;
            continue;
        }

        if input.is_empty() {
            continue;
        }

        // Create and process the task
        info!("Creating task: {}", input);

        match agent.create_and_process_task("User Task", input).await {
            Ok(task_id) => {
                info!("Task completed successfully! Task ID: {}", task_id);
                println!("✅ Task completed: {}", task_id);
            }
            Err(e) => {
                error!("Task failed: {}", e);
                println!("❌ Task failed: {}", e);
            }
        }
    }

    // Graceful shutdown
    info!("Shutting down agent system...");
    agent.shutdown().await?;

    println!("Goodbye!");
    Ok(())
}

async fn show_system_status(agent: &AgentSystem) -> Result<(), Box<dyn std::error::Error>> {
    let status = agent.get_system_status().await?;

    println!("\n📊 System Status:");
    println!("  Health: {}", if status.is_healthy { "✅ Healthy" } else { "❌ Unhealthy" });
    println!("  Tasks: {} total", status.task_stats.total_tasks);
    println!("  Claude: {} available tokens, {} requests",
             status.claude_status.rate_limiter.available_tokens,
             status.claude_status.rate_limiter.available_requests);
    println!("  Sessions: {} active, {} idle",
             status.claude_status.session_stats.active_sessions,
             status.claude_status.session_stats.idle_sessions);

    Ok(())
}
