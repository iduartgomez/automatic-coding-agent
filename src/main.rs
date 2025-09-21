use automatic_coding_agent::{AgentConfig, AgentSystem};
use automatic_coding_agent::cli::{
    Args, ExecutionMode, BatchConfig, InteractiveConfig,
    TaskInput, TaskLoader, ConfigDiscovery,
    args::{show_help, show_version}
};
use std::io::{self, Write};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("automatic_coding_agent=info")
        .init();

    info!("Starting Automatic Coding Agent");

    // Parse command line arguments
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            show_help();
            std::process::exit(1);
        }
    };

    // Execute based on mode
    match args.mode {
        ExecutionMode::Batch(config) => run_batch_mode(config).await,
        ExecutionMode::Interactive(config) => run_interactive_mode(config).await,
        ExecutionMode::Help => {
            show_help();
            Ok(())
        }
        ExecutionMode::Version => {
            show_version();
            Ok(())
        }
        ExecutionMode::ShowConfig => {
            ConfigDiscovery::show_discovery_info();
            Ok(())
        }
    }
}

async fn run_batch_mode(config: BatchConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running in batch mode with task input: {:?}", config.task_input);

    // Discover and load configuration
    let agent_config = if let Some(ref config_override) = config.config_override {
        info!("Loading configuration override from: {:?}", config_override);
        AgentConfig::from_toml_file(config_override)?
    } else {
        info!("Discovering default configuration...");
        let default_config = ConfigDiscovery::discover_config()?;
        default_config.to_agent_config(config.workspace_override.clone())
    };

    // Load and process tasks based on input type
    let tasks = match &config.task_input {
        TaskInput::SingleFile(path) => {
            info!("Loading single task from file: {:?}", path);
            vec![TaskLoader::parse_single_file_task(path)?]
        }
        TaskInput::TaskList(path) => {
            info!("Loading task list from file: {:?}", path);
            let mut tasks = TaskLoader::parse_task_list(path)?;

            // Resolve references
            info!("Resolving task references...");
            TaskLoader::resolve_task_references(&mut tasks)?;

            tasks
        }
        TaskInput::ConfigWithTasks(path) => {
            // This is the legacy TOML format - handle it differently
            info!("Loading legacy TOML configuration with tasks: {:?}", path);
            return run_legacy_config_mode(path.clone(), config).await;
        }
    };

    if config.verbose {
        println!("📁 Loaded {} tasks", tasks.len());
        for (i, task) in tasks.iter().enumerate() {
            let description = if task.description.len() > 100 {
                format!("{}...", &task.description[..97])
            } else {
                task.description.clone()
            };
            println!("  📋 {}: {}", i + 1, description);
            if task.reference_file.is_some() {
                println!("      └─ Has reference file");
            }
        }
    }

    if config.dry_run {
        println!("🔍 Dry run mode - tasks would be executed but won't actually run");
        return Ok(());
    }

    // Convert tasks to agent commands and create final config
    let setup_commands = TaskLoader::tasks_to_agent_commands(tasks);
    let final_agent_config = AgentConfig {
        setup_commands,
        ..agent_config
    };

    // Initialize and run agent system
    info!("Initializing agent system for batch execution...");
    let agent = AgentSystem::new(final_agent_config).await?;

    info!("Agent system initialized successfully!");

    if config.verbose {
        println!("✅ All tasks completed successfully!");
    }

    // Graceful shutdown
    info!("Shutting down agent system...");
    agent.shutdown().await?;

    Ok(())
}

async fn run_legacy_config_mode(
    config_path: std::path::PathBuf,
    config: BatchConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // This handles the old TOML format with embedded tasks
    // Keep for backward compatibility
    info!("Running legacy TOML configuration mode");

    // For now, load it as AgentConfig directly
    let agent_config = AgentConfig::from_toml_file(config_path)?;

    if config.verbose {
        println!("📁 Loaded legacy configuration with {} setup commands", agent_config.setup_commands.len());
    }

    if config.dry_run {
        println!("🔍 Dry run mode - legacy tasks would be executed but won't actually run");
        return Ok(());
    }

    // Initialize and run agent system
    info!("Initializing agent system for legacy batch execution...");
    let agent = AgentSystem::new(agent_config).await?;

    info!("Agent system initialized successfully!");

    if config.verbose {
        println!("✅ All legacy tasks completed successfully!");
    }

    // Graceful shutdown
    info!("Shutting down agent system...");
    agent.shutdown().await?;

    Ok(())
}

async fn run_interactive_mode(config: InteractiveConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running in interactive mode");

    // Discover configuration for interactive mode
    let default_config = ConfigDiscovery::discover_config()?;
    let agent_config = default_config.to_agent_config(config.workspace.clone());

    // Initialize the agent system
    info!("Initializing agent system...");
    let agent = AgentSystem::new(agent_config).await?;

    info!("Agent system initialized successfully!");

    if config.verbose {
        println!("🤖 Interactive mode started. Type 'help' for commands.");
    }

    // Interactive CLI loop (preserved from original)
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

        if input == "help" {
            show_interactive_help();
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

fn show_interactive_help() {
    println!("📖 Interactive Mode Commands:");
    println!("  status  - Show system status");
    println!("  help    - Show this help message");
    println!("  quit    - Exit the application");
    println!("  exit    - Exit the application");
    println!("\n💡 Enter any other text to create and execute a task.");
}

async fn show_system_status(agent: &AgentSystem) -> Result<(), Box<dyn std::error::Error>> {
    let status = agent.get_system_status().await?;

    println!("\n📊 System Status:");
    println!(
        "  Health: {}",
        if status.is_healthy {
            "✅ Healthy"
        } else {
            "❌ Unhealthy"
        }
    );
    println!("  Tasks: {} total", status.task_stats.total_tasks);
    println!(
        "  Claude: {} available tokens, {} requests",
        status.claude_status.rate_limiter.available_tokens,
        status.claude_status.rate_limiter.available_requests
    );
    println!(
        "  Sessions: {} active, {} idle",
        status.claude_status.session_stats.active_sessions,
        status.claude_status.session_stats.idle_sessions
    );

    Ok(())
}