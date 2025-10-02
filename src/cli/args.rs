//! Command line argument parsing
//!
//! This module handles CLI argument parsing with support for:
//! - --task-file: Single file tasks
//! - --tasks: Task list files
//! - --config: Configuration override
//! - --interactive: Interactive mode
//! - Default configuration discovery when no explicit config

use super::tasks::TaskInput;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Configuration file path
    #[arg(short = 'c', long = "config")]
    pub config: Option<PathBuf>,

    /// Single task file to execute
    #[arg(long = "task-file")]
    pub task_file: Option<PathBuf>,

    /// Task list file to execute
    #[arg(long = "tasks")]
    pub tasks: Option<PathBuf>,

    /// Workspace directory
    #[arg(short = 'w', long = "workspace")]
    pub workspace: Option<PathBuf>,

    /// Run in interactive mode
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Run in batch mode (default)
    #[arg(short = 'b', long = "batch")]
    pub batch: bool,

    /// Enable verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Show what would be executed without running
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Use intelligent LLM-based task parser (auto-enabled for complex files)
    #[arg(long = "use-intelligent-parser")]
    pub use_intelligent_parser: bool,

    /// Force naive parser even for complex files
    #[arg(long = "force-naive-parser")]
    pub force_naive_parser: bool,

    /// Context hints for intelligent parser (can be used multiple times)
    #[arg(long = "context", value_name = "HINT")]
    pub context_hints: Vec<String>,

    /// Dump execution plan to file (JSON or TOML format based on extension)
    #[arg(long = "dump-plan", value_name = "FILE")]
    pub dump_plan: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Resume from specific checkpoint ID
    Resume {
        /// Checkpoint ID to resume from
        checkpoint_id: String,
        /// Workspace directory override
        #[arg(short = 'w', long = "workspace")]
        workspace: Option<PathBuf>,
        /// Enable verbose output
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    /// Resume from latest checkpoint
    Continue {
        /// Workspace directory override
        #[arg(short = 'w', long = "workspace")]
        workspace: Option<PathBuf>,
        /// Enable verbose output
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    /// List available checkpoints
    ListCheckpoints {
        /// Include checkpoints from all sessions
        #[arg(long = "all-sessions")]
        all_sessions: bool,
    },
    /// Create manual checkpoint
    CreateCheckpoint {
        /// Checkpoint description
        description: String,
    },
    /// Show configuration discovery information
    ShowConfig,
}

impl Args {
    pub fn parse() -> Self {
        Parser::parse()
    }

    pub fn mode(&self) -> Result<ExecutionMode, String> {
        // Handle subcommands first
        if let Some(command) = &self.command {
            return match command {
                Commands::Resume {
                    checkpoint_id,
                    workspace,
                    verbose,
                } => Ok(ExecutionMode::Resume(ResumeConfig {
                    checkpoint_id: Some(checkpoint_id.clone()),
                    workspace_override: workspace.clone(),
                    verbose: *verbose,
                    continue_latest: false,
                })),
                Commands::Continue { workspace, verbose } => {
                    Ok(ExecutionMode::Resume(ResumeConfig {
                        checkpoint_id: None,
                        workspace_override: workspace.clone(),
                        verbose: *verbose,
                        continue_latest: true,
                    }))
                }
                Commands::ListCheckpoints { all_sessions } => Ok(ExecutionMode::ListCheckpoints {
                    all_sessions: *all_sessions,
                }),
                Commands::CreateCheckpoint { description } => {
                    Ok(ExecutionMode::CreateCheckpoint(description.clone()))
                }
                Commands::ShowConfig => Ok(ExecutionMode::ShowConfig),
            };
        }

        // Handle interactive mode
        if self.interactive {
            return Ok(ExecutionMode::Interactive(InteractiveConfig {
                workspace: self.workspace.clone(),
                verbose: self.verbose,
            }));
        }

        // Handle batch mode (default)
        let task_input = self.determine_task_input()?;

        let mode = ExecutionMode::Batch(BatchConfig {
            task_input,
            config_override: self.config.clone(),
            workspace_override: self.workspace.clone(),
            verbose: self.verbose,
            dry_run: self.dry_run,
            use_intelligent_parser: self.use_intelligent_parser,
            force_naive_parser: self.force_naive_parser,
            context_hints: self.context_hints.clone(),
            dump_plan: self.dump_plan.clone(),
        });

        Ok(mode)
    }

    /// Determine the task input based on provided arguments
    fn determine_task_input(&self) -> Result<TaskInput, String> {
        // Count how many task inputs were provided
        let inputs = [&self.config, &self.task_file, &self.tasks]
            .iter()
            .filter(|x| x.is_some())
            .count();

        match inputs {
            0 => {
                // No explicit task input - this is an error for now
                // In the future, we might support a default task discovery
                Err("task input required (use --task-file, --tasks, or --config)".to_string())
            }
            1 => {
                // Exactly one input - determine which one
                if let Some(path) = &self.task_file {
                    Ok(TaskInput::SingleFile(path.clone()))
                } else if let Some(path) = &self.tasks {
                    Ok(TaskInput::TaskList(path.clone()))
                } else if let Some(path) = &self.config {
                    Ok(TaskInput::ConfigWithTasks(path.clone()))
                } else {
                    unreachable!("One input was counted but none found")
                }
            }
            _ => {
                // Multiple task inputs - this is ambiguous
                Err("Only one task input method can be specified (--task-file, --tasks, or --config)".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_input_determination() {
        // Single task file
        let args = Args {
            command: None,
            config: None,
            task_file: Some(PathBuf::from("task.md")),
            tasks: None,
            workspace: None,
            interactive: false,
            batch: false,
            verbose: false,
            dry_run: false,
            use_intelligent_parser: false,
            force_naive_parser: false,
            context_hints: vec![],
            dump_plan: None,
        };
        let result = args.determine_task_input().unwrap();

        if let TaskInput::SingleFile(path) = result {
            assert_eq!(path, PathBuf::from("task.md"));
        } else {
            panic!("Expected SingleFile");
        }

        // Task list
        let args = Args {
            command: None,
            config: None,
            task_file: None,
            tasks: Some(PathBuf::from("tasks.txt")),
            workspace: None,
            interactive: false,
            batch: false,
            verbose: false,
            dry_run: false,
            use_intelligent_parser: false,
            force_naive_parser: false,
            context_hints: vec![],
            dump_plan: None,
        };
        let result = args.determine_task_input().unwrap();

        if let TaskInput::TaskList(path) = result {
            assert_eq!(path, PathBuf::from("tasks.txt"));
        } else {
            panic!("Expected TaskList");
        }

        // Config with tasks
        let args = Args {
            command: None,
            config: Some(PathBuf::from("config.toml")),
            task_file: None,
            tasks: None,
            workspace: None,
            interactive: false,
            batch: false,
            verbose: false,
            dry_run: false,
            use_intelligent_parser: false,
            force_naive_parser: false,
            context_hints: vec![],
            dump_plan: None,
        };
        let result = args.determine_task_input().unwrap();

        if let TaskInput::ConfigWithTasks(path) = result {
            assert_eq!(path, PathBuf::from("config.toml"));
        } else {
            panic!("Expected ConfigWithTasks");
        }
    }

    #[test]
    fn test_multiple_inputs_error() {
        // Should error with multiple inputs
        let args = Args {
            command: None,
            config: Some(PathBuf::from("config.toml")),
            task_file: Some(PathBuf::from("task.md")),
            tasks: None,
            workspace: None,
            interactive: false,
            batch: false,
            verbose: false,
            dry_run: false,
            use_intelligent_parser: false,
            force_naive_parser: false,
            context_hints: vec![],
            dump_plan: None,
        };
        let result = args.determine_task_input();
        assert!(result.is_err());
    }

    #[test]
    fn test_no_inputs_error() {
        // Should error with no inputs
        let args = Args {
            command: None,
            config: None,
            task_file: None,
            tasks: None,
            workspace: None,
            interactive: false,
            batch: false,
            verbose: false,
            dry_run: false,
            use_intelligent_parser: false,
            force_naive_parser: false,
            context_hints: vec![],
            dump_plan: None,
        };
        let result = args.determine_task_input();
        assert!(result.is_err());
    }
}
