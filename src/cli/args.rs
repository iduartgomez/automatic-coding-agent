//! Command line argument parsing
//!
//! This module handles CLI argument parsing with subcommands:
//! - `run`: Execute a file (auto-detects: task file, task list, or execution plan)
//! - `interactive`: Run in interactive mode
//! - `checkpoint`: Manage checkpoints (list, create, resume)
//! - `show-config`: Show configuration discovery information

use super::tasks::TaskInput;
use crate::llm::types::ProviderType;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProviderCliOption {
    #[clap(name = "claude-code")]
    ClaudeCode,
    #[clap(name = "openai-codex")]
    OpenAICodex,
    Anthropic,
    #[clap(alias = "local")]
    LocalModel,
}

impl ProviderCliOption {
    pub fn into_provider_type(self) -> ProviderType {
        match self {
            ProviderCliOption::ClaudeCode => ProviderType::ClaudeCode,
            ProviderCliOption::OpenAICodex => ProviderType::OpenAICodex,
            ProviderCliOption::Anthropic => ProviderType::Anthropic,
            ProviderCliOption::LocalModel => ProviderType::LocalModel,
        }
    }
}

#[derive(Debug)]
pub enum ExecutionMode {
    Batch(BatchConfig),
    Interactive(InteractiveConfig),
    Resume(ResumeConfig),                   // Resume from checkpoint
    ListCheckpoints { all_sessions: bool }, // List available checkpoints
    CreateCheckpoint(String),               // Create manual checkpoint
    ShowConfig,                             // Show configuration discovery info
}

#[derive(Debug)]
pub struct BatchConfig {
    pub task_input: TaskInput,
    pub config_override: Option<PathBuf>,
    pub workspace_override: Option<PathBuf>,
    pub verbose: bool,
    pub dry_run: bool,
    pub use_intelligent_parser: bool,
    pub force_naive_parser: bool,
    pub context_hints: Vec<String>,
    pub dump_plan: Option<PathBuf>,
    pub provider_override: Option<ProviderType>,
    pub model_override: Option<String>,
    pub use_containers: bool,
    pub container_image: String,
}

#[derive(Debug)]
pub struct InteractiveConfig {
    pub workspace: Option<PathBuf>,
    pub verbose: bool,
}

#[derive(Debug)]
pub struct ResumeConfig {
    pub checkpoint_id: Option<String>, // Specific checkpoint or latest
    pub workspace_override: Option<PathBuf>,
    pub verbose: bool,
    pub continue_latest: bool,
}

#[derive(Debug, Parser)]
#[command(name = "aca")]
#[command(author = "Automatic Coding Agent Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    about = "A Rust-based agentic tool that automates coding tasks using Claude Code in headless mode"
)]
#[command(long_about = None)]
#[command(arg_required_else_help = true)]
pub struct Args {
    /// Override default provider (e.g., Claude) with another configured provider
    #[arg(long = "provider", value_enum, global = true)]
    pub provider: Option<ProviderCliOption>,
    /// Override model to use when invoking the selected provider
    #[arg(long = "model", global = true)]
    pub model: Option<String>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Execute a file (auto-detects task file, task list, or execution plan)
    Run {
        /// Path to file (task, task list, or execution plan)
        file: PathBuf,
        /// Configuration file path
        #[arg(short = 'c', long = "config")]
        config: Option<PathBuf>,
        /// Workspace directory
        #[arg(short = 'w', long = "workspace")]
        workspace: Option<PathBuf>,
        /// Enable verbose output
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
        /// Show what would be executed without running
        #[arg(short = 'n', long = "dry-run")]
        dry_run: bool,
        /// Use intelligent LLM-based task parser (auto-enabled for task lists)
        #[arg(long = "use-intelligent-parser")]
        use_intelligent_parser: bool,
        /// Force naive parser even for complex files
        #[arg(long = "force-naive-parser")]
        force_naive_parser: bool,
        /// Context hints for intelligent parser (can be used multiple times)
        #[arg(long = "context", value_name = "HINT")]
        context_hints: Vec<String>,
        /// Dump execution plan to file (JSON or TOML format based on extension)
        #[arg(long = "dump-plan", value_name = "FILE")]
        dump_plan: Option<PathBuf>,
        /// Execute tasks inside a container (Docker/Podman)
        #[arg(long = "use-containers")]
        use_containers: bool,
        /// Container image to use (default: alpine:latest)
        #[arg(long = "container-image", default_value = "alpine:latest")]
        container_image: String,
    },
    /// Run in interactive mode
    Interactive {
        /// Workspace directory
        #[arg(short = 'w', long = "workspace")]
        workspace: Option<PathBuf>,
        /// Enable verbose output
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    /// Manage checkpoints (list, create, resume)
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommands,
    },
    /// Show configuration discovery information
    ShowConfig,
}

#[derive(Debug, Subcommand)]
pub enum CheckpointCommands {
    /// List available checkpoints
    List {
        /// Include checkpoints from all sessions
        #[arg(long = "all-sessions")]
        all_sessions: bool,
    },
    /// Create a manual checkpoint
    Create {
        /// Checkpoint description
        description: String,
    },
    /// Resume from a checkpoint
    Resume {
        /// Checkpoint ID to resume from
        checkpoint_id: String,
        /// Workspace directory override
        #[arg(short = 'w', long = "workspace")]
        workspace: Option<PathBuf>,
        /// Enable verbose output
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
        /// Resume from latest checkpoint instead of specific ID
        #[arg(long = "latest")]
        latest: bool,
    },
}

impl Args {
    pub fn parse() -> Self {
        Parser::parse()
    }

    pub fn mode(&self) -> Result<ExecutionMode, String> {
        // All execution modes are now handled via subcommands
        match &self.command {
            Some(Commands::Run {
                file,
                config,
                workspace,
                verbose,
                dry_run,
                use_intelligent_parser,
                force_naive_parser,
                context_hints,
                dump_plan,
                use_containers,
                container_image,
            }) => {
                // Auto-detect file type based on extension
                let task_input = Self::detect_file_type(file)?;
                let provider_override = self.provider.map(|p| p.into_provider_type());

                Ok(ExecutionMode::Batch(BatchConfig {
                    task_input,
                    config_override: config.clone(),
                    workspace_override: workspace.clone(),
                    verbose: *verbose,
                    dry_run: *dry_run,
                    use_intelligent_parser: *use_intelligent_parser,
                    force_naive_parser: *force_naive_parser,
                    context_hints: context_hints.clone(),
                    dump_plan: dump_plan.clone(),
                    provider_override,
                    model_override: self.model.clone(),
                    use_containers: *use_containers,
                    container_image: container_image.clone(),
                }))
            }
            Some(Commands::Interactive { workspace, verbose }) => {
                Ok(ExecutionMode::Interactive(InteractiveConfig {
                    workspace: workspace.clone(),
                    verbose: *verbose,
                }))
            }
            Some(Commands::Checkpoint { command }) => match command {
                CheckpointCommands::List { all_sessions } => Ok(ExecutionMode::ListCheckpoints {
                    all_sessions: *all_sessions,
                }),
                CheckpointCommands::Create { description } => {
                    Ok(ExecutionMode::CreateCheckpoint(description.clone()))
                }
                CheckpointCommands::Resume {
                    checkpoint_id,
                    workspace,
                    verbose,
                    latest,
                } => {
                    if *latest {
                        Ok(ExecutionMode::Resume(ResumeConfig {
                            checkpoint_id: None,
                            workspace_override: workspace.clone(),
                            verbose: *verbose,
                            continue_latest: true,
                        }))
                    } else {
                        Ok(ExecutionMode::Resume(ResumeConfig {
                            checkpoint_id: Some(checkpoint_id.clone()),
                            workspace_override: workspace.clone(),
                            verbose: *verbose,
                            continue_latest: false,
                        }))
                    }
                }
            },
            Some(Commands::ShowConfig) => Ok(ExecutionMode::ShowConfig),
            None => {
                Err("No command specified. Use 'aca --help' to see available commands.".to_string())
            }
        }
    }

    /// Auto-detect file type based on extension
    ///
    /// Detection rules:
    /// - `.json` → ExecutionPlan (structured execution plan)
    /// - `.toml` → ConfigWithTasks (TOML config with embedded tasks/plan)
    /// - `.md`, `.txt` → TaskList (markdown task lists)
    /// - Other → TaskList (default)
    ///
    /// Note: Extension matching is case-insensitive
    fn detect_file_type(path: &std::path::Path) -> Result<TaskInput, String> {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "json" => {
                // JSON files are execution plans
                Ok(TaskInput::ExecutionPlan(path.to_path_buf()))
            }
            "toml" => {
                // TOML files can be either execution plans or configs with tasks
                // We use ConfigWithTasks which handles both formats
                Ok(TaskInput::ConfigWithTasks(path.to_path_buf()))
            }
            "md" | "txt" => {
                // Markdown/text files are task lists
                Ok(TaskInput::TaskList(path.to_path_buf()))
            }
            _ => {
                // Default to task list for unknown extensions
                Ok(TaskInput::TaskList(path.to_path_buf()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_command_with_markdown() {
        let args = Args {
            provider: None,
            model: None,
            command: Some(Commands::Run {
                file: PathBuf::from("tasks.md"),
                config: None,
                workspace: None,
                verbose: true,
                dry_run: false,
                use_intelligent_parser: true,
                force_naive_parser: false,
                context_hints: vec!["hint1".to_string()],
                dump_plan: None,
                use_containers: false,
                container_image: "alpine:latest".to_string(),
            }),
        };
        let mode = args.mode().unwrap();

        if let ExecutionMode::Batch(config) = mode {
            assert!(matches!(config.task_input, TaskInput::TaskList(_)));
            assert!(config.verbose);
            assert!(config.use_intelligent_parser);
            assert_eq!(config.context_hints.len(), 1);
        } else {
            panic!("Expected Batch mode");
        }
    }

    #[test]
    fn test_run_command_with_json() {
        let args = Args {
            provider: None,
            model: None,
            command: Some(Commands::Run {
                file: PathBuf::from("plan.json"),
                config: None,
                workspace: None,
                verbose: false,
                dry_run: true,
                use_intelligent_parser: false,
                force_naive_parser: false,
                context_hints: vec![],
                dump_plan: None,
                use_containers: false,
                container_image: "alpine:latest".to_string(),
            }),
        };
        let mode = args.mode().unwrap();

        if let ExecutionMode::Batch(config) = mode {
            assert!(matches!(config.task_input, TaskInput::ExecutionPlan(_)));
            assert!(config.dry_run);
        } else {
            panic!("Expected Batch mode");
        }
    }

    #[test]
    fn test_run_command_with_toml() {
        let args = Args {
            provider: None,
            model: None,
            command: Some(Commands::Run {
                file: PathBuf::from("config.toml"),
                config: None,
                workspace: None,
                verbose: false,
                dry_run: false,
                use_intelligent_parser: false,
                force_naive_parser: false,
                context_hints: vec![],
                dump_plan: None,
                use_containers: false,
                container_image: "alpine:latest".to_string(),
            }),
        };
        let mode = args.mode().unwrap();

        if let ExecutionMode::Batch(config) = mode {
            assert!(matches!(config.task_input, TaskInput::ConfigWithTasks(_)));
        } else {
            panic!("Expected Batch mode");
        }
    }

    #[test]
    fn test_file_type_detection() {
        // Markdown files -> TaskList
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("tasks.md")).unwrap(),
            TaskInput::TaskList(_)
        ));
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("README.md")).unwrap(),
            TaskInput::TaskList(_)
        ));

        // Text files -> TaskList
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("tasks.txt")).unwrap(),
            TaskInput::TaskList(_)
        ));

        // JSON files -> ExecutionPlan
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("plan.json")).unwrap(),
            TaskInput::ExecutionPlan(_)
        ));
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("execution_plan.json")).unwrap(),
            TaskInput::ExecutionPlan(_)
        ));

        // TOML files -> ConfigWithTasks
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("config.toml")).unwrap(),
            TaskInput::ConfigWithTasks(_)
        ));
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from(".aca.toml")).unwrap(),
            TaskInput::ConfigWithTasks(_)
        ));

        // Unknown extension defaults to TaskList
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("unknown.xyz")).unwrap(),
            TaskInput::TaskList(_)
        ));

        // No extension defaults to TaskList
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("tasks")).unwrap(),
            TaskInput::TaskList(_)
        ));
    }

    #[test]
    fn test_file_type_detection_with_paths() {
        // Test with full paths
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("/path/to/tasks.md")).unwrap(),
            TaskInput::TaskList(_)
        ));
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("./relative/plan.json")).unwrap(),
            TaskInput::ExecutionPlan(_)
        ));
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("../config.toml")).unwrap(),
            TaskInput::ConfigWithTasks(_)
        ));
    }

    #[test]
    fn test_file_type_detection_case_sensitivity() {
        // Extensions should be lowercase matched
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("tasks.MD")).unwrap(),
            TaskInput::TaskList(_)
        ));
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("plan.JSON")).unwrap(),
            TaskInput::ExecutionPlan(_)
        ));
        assert!(matches!(
            Args::detect_file_type(&PathBuf::from("config.TOML")).unwrap(),
            TaskInput::ConfigWithTasks(_)
        ));
    }

    #[test]
    fn test_interactive_command() {
        let args = Args {
            provider: None,
            model: None,
            command: Some(Commands::Interactive {
                workspace: Some(PathBuf::from("/workspace")),
                verbose: true,
            }),
        };
        let mode = args.mode().unwrap();

        if let ExecutionMode::Interactive(config) = mode {
            assert!(config.workspace.is_some());
            assert!(config.verbose);
        } else {
            panic!("Expected Interactive mode");
        }
    }

    #[test]
    fn test_no_command_error() {
        let args = Args {
            provider: None,
            model: None,
            command: None,
        };
        let result = args.mode();
        assert!(result.is_err());
    }

    #[test]
    fn test_checkpoint_list() {
        let args = Args {
            provider: None,
            model: None,
            command: Some(Commands::Checkpoint {
                command: CheckpointCommands::List { all_sessions: true },
            }),
        };
        let mode = args.mode().unwrap();

        if let ExecutionMode::ListCheckpoints { all_sessions } = mode {
            assert!(all_sessions);
        } else {
            panic!("Expected ListCheckpoints mode");
        }
    }

    #[test]
    fn test_checkpoint_create() {
        let args = Args {
            provider: None,
            model: None,
            command: Some(Commands::Checkpoint {
                command: CheckpointCommands::Create {
                    description: "Test checkpoint".to_string(),
                },
            }),
        };
        let mode = args.mode().unwrap();

        if let ExecutionMode::CreateCheckpoint(desc) = mode {
            assert_eq!(desc, "Test checkpoint");
        } else {
            panic!("Expected CreateCheckpoint mode");
        }
    }

    #[test]
    fn test_checkpoint_resume_specific() {
        let args = Args {
            provider: None,
            model: None,
            command: Some(Commands::Checkpoint {
                command: CheckpointCommands::Resume {
                    checkpoint_id: "checkpoint-123".to_string(),
                    workspace: Some(PathBuf::from("/workspace")),
                    verbose: true,
                    latest: false,
                },
            }),
        };
        let mode = args.mode().unwrap();

        if let ExecutionMode::Resume(config) = mode {
            assert_eq!(config.checkpoint_id, Some("checkpoint-123".to_string()));
            assert!(!config.continue_latest);
            assert!(config.verbose);
            assert!(config.workspace_override.is_some());
        } else {
            panic!("Expected Resume mode");
        }
    }

    #[test]
    fn test_checkpoint_resume_latest() {
        let args = Args {
            provider: None,
            model: None,
            command: Some(Commands::Checkpoint {
                command: CheckpointCommands::Resume {
                    checkpoint_id: "ignored".to_string(),
                    workspace: None,
                    verbose: false,
                    latest: true,
                },
            }),
        };
        let mode = args.mode().unwrap();

        if let ExecutionMode::Resume(config) = mode {
            assert_eq!(config.checkpoint_id, None);
            assert!(config.continue_latest);
            assert!(!config.verbose);
        } else {
            panic!("Expected Resume mode");
        }
    }
}
