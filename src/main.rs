use automatic_coding_agent::{AgentConfig, AgentSystem};
use lexopt::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Debug)]
enum ExecutionMode {
    Batch(BatchConfig),
    Interactive(InteractiveConfig),
    Help,
    Version,
}

#[derive(Debug)]
struct BatchConfig {
    config_path: PathBuf,
    workspace_override: Option<PathBuf>,
    verbose: bool,
    dry_run: bool,
}

#[derive(Debug)]
struct InteractiveConfig {
    workspace: Option<PathBuf>,
    verbose: bool,
}

#[derive(Debug)]
struct Args {
    mode: ExecutionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskConfig {
    name: String,
    description: Option<String>,
    commands: Vec<String>,
    depends_on: Vec<String>,
    optional: bool,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskFileConfig {
    workspace_path: PathBuf,
    tasks: Vec<TaskConfig>,

    #[serde(flatten)]
    agent_config: AgentConfigBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentConfigBase {
    session_config: automatic_coding_agent::session::SessionManagerConfig,
    task_config: automatic_coding_agent::task::TaskManagerConfig,
    claude_config: automatic_coding_agent::claude::ClaudeConfig,
}

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
    }
}

impl Args {
    fn parse() -> Result<Self, lexopt::Error> {
        let mut parser = lexopt::Parser::from_env();
        let mut config_path: Option<PathBuf> = None;
        let mut workspace: Option<PathBuf> = None;
        let mut verbose = false;
        let mut dry_run = false;
        let mut force_interactive = false;

        while let Some(arg) = parser.next()? {
            match arg {
                Short('c') | Long("config") => {
                    config_path = Some(parser.value()?.parse()?);
                }
                Short('w') | Long("workspace") => {
                    workspace = Some(parser.value()?.parse()?);
                }
                Short('i') | Long("interactive") => {
                    force_interactive = true;
                }
                Short('b') | Long("batch") => {
                    // Explicit batch mode (default anyway)
                }
                Short('v') | Long("verbose") => {
                    verbose = true;
                }
                Short('n') | Long("dry-run") => {
                    dry_run = true;
                }
                Short('h') | Long("help") => {
                    return Ok(Args {
                        mode: ExecutionMode::Help,
                    });
                }
                Short('V') | Long("version") => {
                    return Ok(Args {
                        mode: ExecutionMode::Version,
                    });
                }
                _ => return Err(arg.unexpected()),
            }
        }

        let mode = if force_interactive {
            ExecutionMode::Interactive(InteractiveConfig { workspace, verbose })
        } else {
            let config_path = config_path.ok_or_else(|| lexopt::Error::MissingValue {
                option: Some("--config".to_string()),
            })?;
            ExecutionMode::Batch(BatchConfig {
                config_path,
                workspace_override: workspace,
                verbose,
                dry_run,
            })
        };

        Ok(Args { mode })
    }
}

impl TaskFileConfig {
    fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: TaskFileConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn to_agent_config(self, workspace_override: Option<PathBuf>) -> AgentConfig {
        let workspace_path = workspace_override.unwrap_or(self.workspace_path);

        AgentConfig {
            workspace_path,
            setup_commands: self
                .tasks
                .into_iter()
                .map(|task| task.to_setup_command())
                .collect(),
            session_config: self.agent_config.session_config,
            task_config: self.agent_config.task_config,
            claude_config: self.agent_config.claude_config,
        }
    }
}

impl TaskConfig {
    fn to_setup_command(self) -> automatic_coding_agent::task::SetupCommand {
        use automatic_coding_agent::task::SetupCommand;
        use chrono::Duration;

        let mut cmd = SetupCommand::new(&self.name, &self.commands[0]);

        if self.commands.len() > 1 {
            cmd = cmd.with_args(self.commands[1..].to_vec());
        }

        if self.optional {
            cmd = cmd.optional();
        }

        if let Some(timeout_secs) = self.timeout_seconds {
            cmd = cmd.with_timeout(Duration::seconds(timeout_secs as i64));
        }

        cmd
    }
}

async fn run_batch_mode(config: BatchConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Running in batch mode with config: {:?}",
        config.config_path
    );

    // Load task configuration
    let task_config = TaskFileConfig::from_file(&config.config_path)?;

    if config.verbose {
        println!(
            "📁 Loaded {} tasks from {:?}",
            task_config.tasks.len(),
            config.config_path
        );
        for task in &task_config.tasks {
            println!(
                "  📋 {}: {}",
                task.name,
                task.description.as_deref().unwrap_or("No description")
            );
        }
    }

    if config.dry_run {
        println!("🔍 Dry run mode - tasks would be executed but won't actually run");
        return Ok(());
    }

    // Convert to agent config
    let agent_config = task_config.to_agent_config(config.workspace_override);

    // Initialize and run agent system
    info!("Initializing agent system for batch execution...");
    let agent = AgentSystem::new(agent_config).await?;

    info!("Agent system initialized successfully!");

    // In batch mode, the setup commands will be executed during AgentSystem::new()
    // For now, we just report success
    if config.verbose {
        println!("✅ All tasks completed successfully!");
    }

    // Graceful shutdown
    info!("Shutting down agent system...");
    agent.shutdown().await?;

    Ok(())
}

async fn run_interactive_mode(config: InteractiveConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running in interactive mode");

    // Create agent config
    let mut agent_config = AgentConfig::default();
    if let Some(workspace) = config.workspace {
        agent_config.workspace_path = workspace;
    }

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

fn show_help() {
    println!("Automatic Coding Agent - AI-powered task automation");
    println!();
    println!("USAGE:");
    println!("    {} [OPTIONS]", env!("CARGO_PKG_NAME"));
    println!();
    println!("OPTIONS:");
    println!(
        "    -c, --config <FILE>     Load tasks from configuration file (required for batch mode)"
    );
    println!("    -w, --workspace <DIR>   Override workspace directory");
    println!("    -i, --interactive       Run in interactive mode");
    println!("    -b, --batch             Run in batch mode (default)");
    println!("    -v, --verbose           Enable verbose output");
    println!("    -n, --dry-run           Show what would be executed without running");
    println!("    -h, --help              Show this help message");
    println!("    -V, --version           Show version information");
    println!();
    println!("EXAMPLES:");
    println!(
        "    {} --config tasks.toml                    # Run tasks from config file",
        env!("CARGO_PKG_NAME")
    );
    println!(
        "    {} --config tasks.toml --verbose         # Run with verbose output",
        env!("CARGO_PKG_NAME")
    );
    println!(
        "    {} --interactive                         # Start interactive mode",
        env!("CARGO_PKG_NAME")
    );
    println!(
        "    {} --config tasks.toml --dry-run         # Preview tasks without execution",
        env!("CARGO_PKG_NAME")
    );
}

fn show_version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
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
