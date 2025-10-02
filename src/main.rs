use aca::cli::{
    Args, BatchConfig, ConfigDiscovery, ExecutionMode, InteractiveConfig, TaskInput, TaskLoader,
    args::ResumeConfig,
};
use aca::task::ExecutionPlan;
use aca::env;
use aca::session::persistence::PersistenceConfig;
use aca::session::recovery::RecoveryConfig;
use aca::session::{SessionInitOptions, SessionManager, SessionManagerConfig};
use aca::task::TaskStatus;
use aca::task::manager::TaskManagerConfig;
use aca::{AgentConfig, AgentSystem};
use std::io::{self, Write};
use tracing::{error, info};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("automatic_coding_agent=info")
        .init();

    info!("Starting Automatic Coding Agent");

    // Parse command line arguments
    let args = Args::parse();

    // Execute based on mode
    let mode = match args.mode() {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    match mode {
        ExecutionMode::Batch(config) => run_batch_mode(config).await,
        ExecutionMode::Interactive(config) => run_interactive_mode(config).await,
        ExecutionMode::Resume(config) => run_resume_mode(config).await,
        ExecutionMode::ListCheckpoints { all_sessions } => {
            list_available_checkpoints(all_sessions).await
        }
        ExecutionMode::CreateCheckpoint(description) => create_manual_checkpoint(description).await,
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

    // Convert task input to execution plan
    let execution_plan = match &config.task_input {
        TaskInput::ConfigWithTasks(path) => {
            // This is the structured TOML configuration with tasks - handle it differently
            info!(
                "Loading structured TOML configuration with tasks: {:?}",
                path
            );
            return run_structured_config_mode(path.clone(), config).await;
        }
        _ => {
            info!("Converting task input to execution plan...");

            // Determine whether to use intelligent parser
            let use_intelligent = if config.force_naive_parser {
                false
            } else if config.use_intelligent_parser {
                true
            } else {
                // Auto-detect: use intelligent parser for task lists
                matches!(config.task_input, TaskInput::TaskList(_))
            };

            if use_intelligent {
                info!("Using intelligent LLM-based task parser");
                TaskLoader::task_input_to_execution_plan_with_options(
                    &config.task_input,
                    true,
                    config.context_hints.clone(),
                )
                .await?
            } else {
                TaskLoader::task_input_to_execution_plan(&config.task_input)?
            }
        }
    };

    if config.verbose {
        println!("📁 Created execution plan: {}", execution_plan.summary());
        if let Some(ref name) = execution_plan.metadata.name {
            println!("  📋 Plan: {}", name);
        }
        if let Some(ref description) = execution_plan.metadata.description {
            println!("  📝 Description: {}", description);
        }
        if execution_plan.has_setup_commands() {
            println!(
                "  ⚙️  Setup commands: {}",
                execution_plan.setup_command_count()
            );
        }
        if execution_plan.has_tasks() {
            println!("  🎯 Tasks: {}", execution_plan.task_count());
            for (i, task_spec) in execution_plan.task_specs.iter().enumerate() {
                let title = if task_spec.title.len() > 80 {
                    format!("{}...", &task_spec.title[..77])
                } else {
                    task_spec.title.clone()
                };
                println!("      {}. {}", i + 1, title);
            }
        }
    }

    // Dump execution plan if requested
    if let Some(ref dump_path) = config.dump_plan {
        dump_execution_plan(&execution_plan, dump_path)?;
        println!("📄 Execution plan dumped to: {}", dump_path.display());
        if config.dry_run {
            return Ok(());
        }
    }

    if config.dry_run {
        println!("🔍 Dry run mode - execution plan would be processed but won't actually run");
        return Ok(());
    }

    // Initialize agent system
    info!("Initializing agent system for batch execution...");
    let agent = AgentSystem::new(agent_config).await?;

    info!("Agent system initialized successfully!");

    // Execute the plan using the unified execution path
    info!("Executing plan with unified agent system...");
    let task_ids = agent.execute_plan(execution_plan).await?;

    if config.verbose {
        if !task_ids.is_empty() {
            println!(
                "✅ All {} tasks in plan completed successfully!",
                task_ids.len()
            );
            if task_ids.len() <= 5 {
                // Show task IDs for small numbers of tasks
                for (i, task_id) in task_ids.iter().enumerate() {
                    println!("    {}. {}", i + 1, task_id);
                }
            }
        } else {
            println!("ℹ️  No tasks were executed (setup-only plan)");
        }
    }

    // Graceful shutdown
    info!("Shutting down agent system...");
    agent.shutdown().await?;

    Ok(())
}

async fn run_structured_config_mode(
    config_path: std::path::PathBuf,
    config: BatchConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // This handles the structured TOML format with embedded tasks and configuration
    info!("Running structured TOML configuration mode");

    // Load the agent config from TOML file
    let agent_config = AgentConfig::from_toml_file(config_path)?;

    // Convert the agent config to execution plan
    info!("Converting structured configuration to execution plan...");
    let execution_plan = AgentSystem::agent_config_to_execution_plan(&agent_config);

    if config.verbose {
        println!(
            "📁 Created execution plan from structured config: {}",
            execution_plan.summary()
        );
        if let Some(ref name) = execution_plan.metadata.name {
            println!("  📋 Plan: {}", name);
        }
        if let Some(ref description) = execution_plan.metadata.description {
            println!("  📝 Description: {}", description);
        }
        if execution_plan.has_setup_commands() {
            println!(
                "  ⚙️  Setup commands: {}",
                execution_plan.setup_command_count()
            );
            for (i, setup_cmd) in execution_plan.setup_commands.iter().enumerate() {
                println!(
                    "      {}. {} ({})",
                    i + 1,
                    setup_cmd.name,
                    setup_cmd.command
                );
            }
        }
    }

    if config.dry_run {
        println!(
            "🔍 Dry run mode - structured execution plan would be processed but won't actually run"
        );
        return Ok(());
    }

    // Initialize agent system
    info!("Initializing agent system for structured batch execution...");
    let agent = AgentSystem::new(agent_config).await?;

    info!("Agent system initialized successfully!");

    // Execute the plan using the unified execution path
    info!("Executing structured configuration plan...");
    let task_ids = agent.execute_plan(execution_plan).await?;

    if config.verbose {
        if !task_ids.is_empty() {
            println!(
                "✅ All {} tasks in structured plan completed successfully!",
                task_ids.len()
            );
        } else {
            println!("✅ Structured configuration setup completed successfully!");
        }
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

    let workspace = config
        .workspace_override
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    // Check if .aca directory structure exists
    let _aca_dir = env::aca_dir_path(&workspace);
    let sessions_dir = env::sessions_dir_path(&workspace);

    if !sessions_dir.exists() {
        eprintln!(
            "Error: No session data found in directory: {}",
            workspace.display()
        );
        eprintln!("Make sure you're in the correct workspace directory.");
        std::process::exit(1);
    }

    // Determine which checkpoint to restore from
    let checkpoint_id = if config.continue_latest {
        match find_latest_checkpoint(&workspace).await {
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
            println!(
                "🔄 Found {} incomplete tasks. Continuing processing...",
                incomplete_tasks.len()
            );
        }

        let mut successful_tasks = 0;
        let total_tasks = incomplete_tasks.len();

        for (task_num, task_id) in incomplete_tasks.iter().enumerate() {
            if config.verbose {
                println!(
                    "🔄 Processing incomplete task {}/{}: {}",
                    task_num + 1,
                    total_tasks,
                    task_id
                );
            }

            match agent.process_task(*task_id).await {
                Ok(()) => {
                    info!(
                        "Resumed task {}/{} completed successfully! Task ID: {}",
                        task_num + 1,
                        total_tasks,
                        task_id
                    );
                    if config.verbose {
                        println!(
                            "✅ Task {}/{} completed: {}",
                            task_num + 1,
                            total_tasks,
                            task_id
                        );
                    }
                    successful_tasks += 1;
                }
                Err(e) => {
                    error!("Failed to process resumed task {}: {}", task_id, e);
                    if config.verbose {
                        println!("❌ Task {}/{} failed: {}", task_num + 1, total_tasks, e);
                    }
                }
            }
        }

        if successful_tasks == total_tasks {
            println!(
                "✅ All {} resumed tasks completed successfully!",
                total_tasks
            );
        } else {
            println!(
                "⚠️  {}/{} resumed tasks completed successfully",
                successful_tasks, total_tasks
            );
        }
    } else if config.verbose {
        println!("ℹ️  No incomplete tasks found. Session restored successfully.");
    }

    // Graceful shutdown
    info!("Shutting down agent system...");
    agent.shutdown().await?;

    Ok(())
}

async fn list_available_checkpoints(_all_sessions: bool) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = std::env::current_dir()?;
    let _aca_dir = env::aca_dir_path(&workspace);
    let sessions_dir = env::sessions_dir_path(&workspace);

    // Check if .aca directory structure exists
    if !sessions_dir.exists() {
        println!("No session data found in current directory.");
        println!("Make sure you're in a workspace that has been used with aca.");
        return Ok(());
    }

    // Find the most recent session directory
    let mut latest_session_dir = None;
    let mut latest_time = None;

    if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Ok(metadata) = entry.metadata()
                && let Ok(modified) = metadata.modified()
                && (latest_time.is_none() || Some(modified) > latest_time)
            {
                latest_time = Some(modified);
                latest_session_dir = Some(path);
            }
        }
    }

    let Some(_session_dir) = latest_session_dir else {
        println!("No session directories found in .aca/sessions/");
        return Ok(());
    };

    // Create a temporary session manager to list all checkpoints across sessions
    let session_config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Temporary Session for Listing".to_string(),
        description: Some("Temporary session for listing checkpoints".to_string()),
        workspace_root: workspace.clone(),
        task_manager_config: TaskManagerConfig::default(),
        persistence_config: PersistenceConfig::default(),
        recovery_config: RecoveryConfig::default(),
        enable_auto_save: false,
        restore_from_checkpoint: None,
    };

    let temp_session =
        match SessionManager::new(workspace.clone(), session_config, init_options).await {
            Ok(session) => session,
            Err(e) => {
                eprintln!("Error: Failed to create session for listing: {}", e);
                std::process::exit(1);
            }
        };

    // For CLI usage, default to showing all checkpoints across sessions
    // The --all-sessions flag is currently redundant but kept for explicit behavior
    let show_all = true; // CLI users expect to see all checkpoints by default
    let checkpoints = temp_session.list_checkpoints(show_all).await?;

    if checkpoints.is_empty() {
        println!("No checkpoints available in current workspace.");
    } else {
        println!("Available checkpoints in {}:", workspace.display());
        println!();
        for checkpoint in checkpoints {
            println!(
                "📌 {} ({})",
                checkpoint.id,
                checkpoint.created_at.format("%Y-%m-%d %H:%M:%S")
            );
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
    let _aca_dir = env::aca_dir_path(&workspace);
    let sessions_dir = env::sessions_dir_path(&workspace);

    // Check if .aca directory structure exists
    if !sessions_dir.exists() {
        eprintln!("Error: No active session found in current directory.");
        eprintln!("Start a task first to create a session, then you can create checkpoints.");
        std::process::exit(1);
    }

    // Create checkpoint using a temporary session manager that operates on the latest session
    let session_config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Temporary Session for Manual Checkpoint".to_string(),
        description: Some("Temporary session for creating manual checkpoint".to_string()),
        workspace_root: workspace.clone(),
        task_manager_config: TaskManagerConfig::default(),
        persistence_config: PersistenceConfig::default(),
        recovery_config: RecoveryConfig::default(),
        enable_auto_save: false,
        restore_from_checkpoint: None,
    };

    let temp_session =
        match SessionManager::new(workspace.clone(), session_config, init_options).await {
            Ok(session) => session,
            Err(e) => {
                eprintln!("Error: Failed to create session for checkpoint: {}", e);
                std::process::exit(1);
            }
        };

    let checkpoint = match temp_session
        .create_checkpoint_in_latest_session_of_workspace(description.clone())
        .await
    {
        Ok(checkpoint) => checkpoint,
        Err(e) => {
            eprintln!("Error: Failed to create checkpoint: {}", e);
            std::process::exit(1);
        }
    };

    println!("✅ Checkpoint created: {}", checkpoint.id);
    println!("   Description: {}", description);
    println!(
        "   Created: {}",
        checkpoint.created_at.format("%Y-%m-%d %H:%M:%S")
    );

    Ok(())
}

async fn find_latest_checkpoint(
    session_dir: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
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
    let temp_session =
        SessionManager::new(session_dir.to_path_buf(), session_config, init_options).await?;
    let checkpoints = temp_session.list_checkpoints(true).await?;

    if checkpoints.is_empty() {
        return Err("No checkpoints available".into());
    }

    // Find the most recent checkpoint
    let latest = checkpoints.iter().max_by_key(|c| c.created_at).unwrap();

    Ok(latest.id.clone())
}

/// Dump execution plan to JSON or TOML file
fn dump_execution_plan(plan: &ExecutionPlan, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("json");

    match extension {
        "json" => {
            let json = serde_json::to_string_pretty(plan)?;
            std::fs::write(path, json)?;
        }
        "toml" => {
            let toml = toml::to_string_pretty(plan)?;
            std::fs::write(path, toml)?;
        }
        _ => {
            return Err(format!(
                "Unsupported format: {}. Use .json or .toml extension",
                extension
            )
            .into());
        }
    }

    Ok(())
}

/// Find incomplete tasks that should be continued when resuming
async fn find_incomplete_tasks(
    agent: &AgentSystem,
) -> Result<Vec<uuid::Uuid>, Box<dyn std::error::Error>> {
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

    info!(
        "Found {} incomplete tasks for resume: {} in-progress, {} eligible",
        incomplete_tasks.len(),
        in_progress_count,
        eligible_count
    );

    Ok(incomplete_tasks)
}
