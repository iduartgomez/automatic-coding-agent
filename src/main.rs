use automatic_coding_agent::cli::{
    Args, BatchConfig, ConfigDiscovery, ExecutionMode, InteractiveConfig, TaskInput, TaskLoader,
    args::{show_help, show_version, ResumeConfig},
};
use automatic_coding_agent::{AgentConfig, AgentSystem};
use automatic_coding_agent::session::{SessionManager, SessionManagerConfig, SessionInitOptions};
use automatic_coding_agent::session::persistence::PersistenceConfig;
use automatic_coding_agent::session::recovery::RecoveryConfig;
use automatic_coding_agent::task::manager::TaskManagerConfig;
use automatic_coding_agent::task::TaskStatus;
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
        ExecutionMode::Resume(config) => run_resume_mode(config).await,
        ExecutionMode::ListCheckpoints => list_available_checkpoints().await,
        ExecutionMode::CreateCheckpoint(description) => create_manual_checkpoint(description).await,
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
    info!(
        "Running in batch mode with task input: {:?}",
        config.task_input
    );

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

    // Initialize agent system
    info!("Initializing agent system for batch execution...");
    let agent = AgentSystem::new(agent_config).await?;

    info!("Agent system initialized successfully!");

    // Process each task using the same pattern as interactive mode
    let mut successful_tasks = 0;
    let total_tasks = tasks.len();

    for (i, task) in tasks.into_iter().enumerate() {
        let task_num = i + 1;
        info!(
            "Processing task {}/{}: {}",
            task_num, total_tasks, task.description
        );

        if config.verbose {
            println!(
                "🔄 Processing task {}/{}: {}",
                task_num,
                total_tasks,
                if task.description.len() > 100 {
                    format!("{}...", &task.description[..97])
                } else {
                    task.description.clone()
                }
            );
        }

        match agent
            .create_and_process_task(&format!("Batch Task {}", task_num), &task.description)
            .await
        {
            Ok(task_id) => {
                info!(
                    "Task {}/{} completed successfully! Task ID: {}",
                    task_num, total_tasks, task_id
                );
                if config.verbose {
                    println!(
                        "✅ Task {}/{} completed: {}",
                        task_num, total_tasks, task_id
                    );
                }
                successful_tasks += 1;
            }
            Err(e) => {
                error!("Task {}/{} failed: {}", task_num, total_tasks, e);
                if config.verbose {
                    println!("❌ Task {}/{} failed: {}", task_num, total_tasks, e);
                }
            }
        }
    }

    if config.verbose {
        if successful_tasks == total_tasks {
            println!("✅ All {} tasks completed successfully!", total_tasks);
        } else {
            println!(
                "⚠️  {}/{} tasks completed successfully",
                successful_tasks, total_tasks
            );
        }
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
        println!(
            "📁 Loaded legacy configuration with {} setup commands",
            agent_config.setup_commands.len()
        );
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

async fn run_resume_mode(config: ResumeConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running in resume mode");

    let workspace = config.workspace_override
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let session_dir = workspace.clone(); // Session data is stored in workspace root

    // Check if session data exists
    let session_file = session_dir.join("session.json");
    if !session_file.exists() {
        eprintln!("Error: No session data found in directory: {}", workspace.display());
        eprintln!("Make sure you're in the correct workspace directory.");
        std::process::exit(1);
    }

    // Determine which checkpoint to restore from
    let checkpoint_id = if config.continue_latest {
        match find_latest_checkpoint(&session_dir).await {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Error: Failed to find latest checkpoint: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(id) = config.checkpoint_id {
        id
    } else {
        eprintln!("Error: Must specify --resume <checkpoint-id> or --continue");
        std::process::exit(1);
    };

    if config.verbose {
        println!("🔄 Resuming from checkpoint: {}", checkpoint_id);
    }

    // Discover and load configuration
    let default_config = ConfigDiscovery::discover_config()?;
    let agent_config = default_config.to_agent_config(Some(workspace.clone()));

    // Initialize agent system with restore
    info!("Initializing agent system with checkpoint restore...");
    // TODO: We need to modify AgentSystem to support session restore
    // For now, we'll use the regular initialization and these variables are placeholders for future use
    let _session_config = SessionManagerConfig::default();
    let _init_options = SessionInitOptions {
        name: "Resumed Session".to_string(),
        description: Some("Session resumed from checkpoint".to_string()),
        workspace_root: workspace.clone(),
        task_manager_config: TaskManagerConfig::default(),
        persistence_config: PersistenceConfig::default(),
        recovery_config: RecoveryConfig::default(),
        enable_auto_save: true,
        restore_from_checkpoint: Some(checkpoint_id.clone()),
    };

    let agent = AgentSystem::new(agent_config).await?;

    if config.verbose {
        println!("✅ Successfully resumed from checkpoint: {}", checkpoint_id);
        println!("🤖 Agent system ready. Checking for incomplete tasks...");
    }

    // Check for incomplete tasks and continue processing
    let incomplete_tasks = find_incomplete_tasks(&agent).await?;

    if !incomplete_tasks.is_empty() {
        if config.verbose {
            println!("🔄 Found {} incomplete tasks. Continuing processing...", incomplete_tasks.len());
        }

        let mut successful_tasks = 0;
        let total_tasks = incomplete_tasks.len();

        for (task_num, task_id) in incomplete_tasks.iter().enumerate() {
            if config.verbose {
                println!(
                    "🔄 Processing incomplete task {}/{}: {}",
                    task_num + 1, total_tasks, task_id
                );
            }

            match agent.process_task(*task_id).await {
                Ok(()) => {
                    info!("Resumed task {}/{} completed successfully! Task ID: {}",
                         task_num + 1, total_tasks, task_id);
                    if config.verbose {
                        println!("✅ Task {}/{} completed: {}",
                               task_num + 1, total_tasks, task_id);
                    }
                    successful_tasks += 1;
                }
                Err(e) => {
                    error!("Failed to process resumed task {}: {}", task_id, e);
                    if config.verbose {
                        println!("❌ Task {}/{} failed: {}",
                               task_num + 1, total_tasks, e);
                    }
                }
            }
        }

        if successful_tasks == total_tasks {
            println!("✅ All {} resumed tasks completed successfully!", total_tasks);
        } else {
            println!("⚠️  {}/{} resumed tasks completed successfully",
                    successful_tasks, total_tasks);
        }
    } else if config.verbose {
        println!("ℹ️  No incomplete tasks found. Session restored successfully.");
    }

    // Graceful shutdown
    info!("Shutting down agent system...");
    agent.shutdown().await?;

    Ok(())
}

async fn list_available_checkpoints() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = std::env::current_dir()?;
    let session_dir = workspace.clone(); // Session data is stored in workspace root

    // Check if session data exists (look for session.json or checkpoint files)
    let session_file = session_dir.join("session.json");
    let has_checkpoints = std::fs::read_dir(&session_dir)
        .map(|mut entries| {
            entries.any(|entry| {
                entry.map(|e| e.file_name().to_string_lossy().starts_with("checkpoint_"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !session_file.exists() && !has_checkpoints {
        println!("No session data found in current directory.");
        println!("Make sure you're in a workspace that has been used with the automatic-coding-agent.");
        return Ok(());
    }

    // Load existing session to list checkpoints
    let session_config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Temporary Session".to_string(),
        description: Some("Temporary session for listing checkpoints".to_string()),
        workspace_root: workspace.clone(),
        task_manager_config: TaskManagerConfig::default(),
        persistence_config: PersistenceConfig::default(),
        recovery_config: RecoveryConfig::default(),
        enable_auto_save: false,
        restore_from_checkpoint: None,
    };
    let temp_session = match SessionManager::new(session_dir, session_config, init_options).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("Error: Failed to load session data: {}", e);
            std::process::exit(1);
        }
    };

    let checkpoints = temp_session.list_checkpoints().await?;

    if checkpoints.is_empty() {
        println!("No checkpoints available in current workspace.");
    } else {
        println!("Available checkpoints in {}:", workspace.display());
        println!();
        for checkpoint in checkpoints {
            println!("📌 {} ({})", checkpoint.id, checkpoint.created_at.format("%Y-%m-%d %H:%M:%S"));
            println!("   Description: {}", checkpoint.description);
            if checkpoint.task_count > 0 {
                println!("   Tasks: {} total", checkpoint.task_count);
            }
            println!();
        }
        println!("Use --resume <checkpoint-id> to restore from a specific checkpoint");
        println!("Use --continue to resume from the latest checkpoint");
    }

    Ok(())
}

async fn create_manual_checkpoint(description: String) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = std::env::current_dir()?;
    let session_dir = workspace.clone(); // Session data is stored in workspace root

    // Check if session data exists
    let session_file = session_dir.join("session.json");
    if !session_file.exists() {
        eprintln!("Error: No active session found in current directory.");
        eprintln!("Start a task first to create a session, then you can create checkpoints.");
        std::process::exit(1);
    }

    // Load existing session to create checkpoint
    let session_config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Temporary Session".to_string(),
        description: Some("Temporary session for creating checkpoint".to_string()),
        workspace_root: workspace.clone(),
        task_manager_config: TaskManagerConfig::default(),
        persistence_config: PersistenceConfig::default(),
        recovery_config: RecoveryConfig::default(),
        enable_auto_save: false,
        restore_from_checkpoint: None,
    };
    let session_manager = match SessionManager::new(session_dir, session_config, init_options).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("Error: Failed to load session data: {}", e);
            std::process::exit(1);
        }
    };

    let checkpoint = session_manager.create_checkpoint(description.clone()).await?;

    println!("✅ Checkpoint created: {}", checkpoint.id);
    println!("   Description: {}", description);
    println!("   Created: {}", checkpoint.created_at.format("%Y-%m-%d %H:%M:%S"));

    Ok(())
}

async fn find_latest_checkpoint(session_dir: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    let session_config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Temporary Session".to_string(),
        description: Some("Temporary session for finding latest checkpoint".to_string()),
        workspace_root: session_dir.to_path_buf(),
        task_manager_config: TaskManagerConfig::default(),
        persistence_config: PersistenceConfig::default(),
        recovery_config: RecoveryConfig::default(),
        enable_auto_save: false,
        restore_from_checkpoint: None,
    };
    let temp_session = SessionManager::new(session_dir.to_path_buf(), session_config, init_options).await?;
    let checkpoints = temp_session.list_checkpoints().await?;

    if checkpoints.is_empty() {
        return Err("No checkpoints available".into());
    }

    // Find the most recent checkpoint
    let latest = checkpoints
        .iter()
        .max_by_key(|c| c.created_at)
        .unwrap();

    Ok(latest.id.clone())
}

/// Find incomplete tasks that should be continued when resuming
async fn find_incomplete_tasks(agent: &AgentSystem) -> Result<Vec<uuid::Uuid>, Box<dyn std::error::Error>> {
    let task_manager = agent.task_manager();

    // Look for tasks that are in progress or eligible to be processed
    let in_progress_tasks = task_manager
        .get_tasks_by_status(|status| matches!(status, TaskStatus::InProgress { .. }))
        .await?;

    let eligible_tasks = task_manager.get_eligible_tasks().await?;

    // Combine both sets, prioritizing in-progress tasks
    let mut incomplete_tasks = Vec::new();

    // Track counts before moving
    let in_progress_count = in_progress_tasks.len();
    let eligible_count = eligible_tasks.len();

    // Add in-progress tasks first (highest priority for resume)
    incomplete_tasks.extend(in_progress_tasks);

    // Add eligible tasks that aren't already in progress
    for task_id in &eligible_tasks {
        if !incomplete_tasks.contains(task_id) {
            incomplete_tasks.push(*task_id);
        }
    }

    info!("Found {} incomplete tasks for resume: {} in-progress, {} eligible",
          incomplete_tasks.len(),
          in_progress_count,
          eligible_count);

    Ok(incomplete_tasks)
}
