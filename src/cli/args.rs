//! Command line argument parsing
//!
//! This module handles CLI argument parsing with support for:
//! - --task-file: Single file tasks
//! - --tasks: Task list files
//! - --config: Configuration override
//! - --interactive: Interactive mode
//! - Default configuration discovery when no explicit config

use super::tasks::TaskInput;
use lexopt::prelude::*;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug)]
pub enum ExecutionMode {
    Batch(BatchConfig),
    Interactive(InteractiveConfig),
    Resume(ResumeConfig),     // Resume from checkpoint
    ListCheckpoints,          // List available checkpoints
    CreateCheckpoint(String), // Create manual checkpoint
    Help,
    Version,
    ShowConfig,
}

#[derive(Debug)]
pub struct BatchConfig {
    pub task_input: TaskInput,
    pub config_override: Option<PathBuf>,
    pub workspace_override: Option<PathBuf>,
    pub verbose: bool,
    pub dry_run: bool,
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

#[derive(Debug)]
pub struct Args {
    pub mode: ExecutionMode,
}

impl Args {
    pub fn parse() -> Result<Self, lexopt::Error> {
        let mut parser = lexopt::Parser::from_env();
        let mut config_path: Option<PathBuf> = None;
        let mut task_file: Option<PathBuf> = None;
        let mut tasks_file: Option<PathBuf> = None;
        let mut workspace: Option<PathBuf> = None;
        let mut verbose = false;
        let mut dry_run = false;
        let mut force_interactive = false;
        let mut show_config = false;
        let mut resume_checkpoint: Option<String> = None;
        let mut continue_latest = false;
        let mut list_checkpoints = false;
        let mut create_checkpoint: Option<String> = None;

        while let Some(arg) = parser.next()? {
            match arg {
                // Configuration and task input
                Short('c') | Long("config") => {
                    config_path = Some(parser.value()?.parse()?);
                }
                Long("task-file") => {
                    task_file = Some(parser.value()?.parse()?);
                }
                Long("tasks") => {
                    tasks_file = Some(parser.value()?.parse()?);
                }

                // Workspace and execution options
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

                // Information commands
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
                Long("show-config") => {
                    show_config = true;
                }

                // Resume functionality
                Long("resume") => {
                    resume_checkpoint = Some(parser.value()?.to_string_lossy().to_string());
                }
                Long("continue") => {
                    continue_latest = true;
                }
                Long("list-checkpoints") => {
                    list_checkpoints = true;
                }
                Long("create-checkpoint") => {
                    create_checkpoint = Some(parser.value()?.to_string_lossy().to_string());
                }

                _ => return Err(arg.unexpected()),
            }
        }

        // Handle special modes first
        if show_config {
            return Ok(Args {
                mode: ExecutionMode::ShowConfig,
            });
        }

        if list_checkpoints {
            return Ok(Args {
                mode: ExecutionMode::ListCheckpoints,
            });
        }

        if let Some(description) = create_checkpoint {
            return Ok(Args {
                mode: ExecutionMode::CreateCheckpoint(description),
            });
        }

        if resume_checkpoint.is_some() || continue_latest {
            return Ok(Args {
                mode: ExecutionMode::Resume(ResumeConfig {
                    checkpoint_id: resume_checkpoint,
                    workspace_override: workspace,
                    verbose,
                    continue_latest,
                }),
            });
        }

        if force_interactive {
            return Ok(Args {
                mode: ExecutionMode::Interactive(InteractiveConfig { workspace, verbose }),
            });
        }

        // Determine task input for batch mode
        let task_input = Self::determine_task_input(config_path.clone(), task_file, tasks_file)?;

        let mode = ExecutionMode::Batch(BatchConfig {
            task_input,
            config_override: config_path,
            workspace_override: workspace,
            verbose,
            dry_run,
        });

        debug!("Parsed CLI arguments: {:?}", mode);

        Ok(Args { mode })
    }

    /// Determine the task input based on provided arguments
    fn determine_task_input(
        config_path: Option<PathBuf>,
        task_file: Option<PathBuf>,
        tasks_file: Option<PathBuf>,
    ) -> Result<TaskInput, lexopt::Error> {
        // Count how many task inputs were provided
        let inputs = [&config_path, &task_file, &tasks_file]
            .iter()
            .filter(|x| x.is_some())
            .count();

        match inputs {
            0 => {
                // No explicit task input - this is an error for now
                // In the future, we might support a default task discovery
                Err(lexopt::Error::MissingValue {
                    option: Some("task input (use --task-file, --tasks, or --config)".to_string()),
                })
            }
            1 => {
                // Exactly one input - determine which one
                if let Some(path) = task_file {
                    Ok(TaskInput::SingleFile(path))
                } else if let Some(path) = tasks_file {
                    Ok(TaskInput::TaskList(path))
                } else if let Some(path) = config_path {
                    Ok(TaskInput::ConfigWithTasks(path))
                } else {
                    unreachable!("One input was counted but none found")
                }
            }
            _ => {
                // Multiple task inputs - this is ambiguous
                Err(lexopt::Error::Custom(
                    "Only one task input method can be specified (--task-file, --tasks, or --config)".into()
                ))
            }
        }
    }
}

/// Display help information
pub fn show_help() {
    println!("Automatic Coding Agent - AI-powered task automation");
    println!();
    println!("USAGE:");
    println!("    {} [OPTIONS]", env!("CARGO_PKG_NAME"));
    println!();
    println!("TASK INPUT OPTIONS (choose one):");
    println!("        --task-file <FILE>      Execute a single task from any UTF-8 file");
    println!("        --tasks <FILE>          Execute multiple tasks from a task list file");
    println!("    -c, --config <FILE>         Load tasks from TOML configuration file (legacy)");
    println!();
    println!("EXECUTION OPTIONS:");
    println!("    -w, --workspace <DIR>       Override workspace directory");
    println!("    -i, --interactive           Run in interactive mode");
    println!("    -b, --batch                 Run in batch mode (default)");
    println!("    -v, --verbose               Enable verbose output");
    println!("    -n, --dry-run               Show what would be executed without running");
    println!();
    println!("RESUME OPTIONS:");
    println!("        --resume <CHECKPOINT>   Resume from specific checkpoint ID");
    println!("        --continue              Resume from latest checkpoint");
    println!("        --list-checkpoints      List available checkpoints");
    println!("        --create-checkpoint <DESC> Create manual checkpoint");
    println!();
    println!("INFORMATION OPTIONS:");
    println!("    -h, --help                  Show this help message");
    println!("    -V, --version               Show version information");
    println!("        --show-config           Show configuration discovery information");
    println!();
    println!("EXAMPLES:");
    println!();
    println!("  # Execute a single task from any text file");
    println!(
        "    {} --task-file implement_auth.md",
        env!("CARGO_PKG_NAME")
    );
    println!("    {} --task-file bug_report.txt", env!("CARGO_PKG_NAME"));
    println!("    {} --task-file requirements", env!("CARGO_PKG_NAME"));
    println!();
    println!("  # Execute multiple tasks from a list");
    println!("    {} --tasks project_todos.md", env!("CARGO_PKG_NAME"));
    println!("    {} --tasks task_list.org", env!("CARGO_PKG_NAME"));
    println!();
    println!("  # Use with options");
    println!(
        "    {} --task-file task.md --verbose --dry-run",
        env!("CARGO_PKG_NAME")
    );
    println!(
        "    {} --tasks todos.txt --workspace /path/to/project",
        env!("CARGO_PKG_NAME")
    );
    println!();
    println!("  # Interactive mode");
    println!("    {} --interactive", env!("CARGO_PKG_NAME"));
    println!();
    println!("  # Resume operations");
    println!("    {} --list-checkpoints", env!("CARGO_PKG_NAME"));
    println!(
        "    {} --resume checkpoint-abc123 --workspace .",
        env!("CARGO_PKG_NAME")
    );
    println!("    {} --continue --workspace .", env!("CARGO_PKG_NAME"));
    println!();
    println!("  # Legacy TOML config format");
    println!("    {} --config full_config.toml", env!("CARGO_PKG_NAME"));
    println!();
    println!("CONFIGURATION:");
    println!("  The tool searches for default configuration in this order:");
    println!("    1. ./aca.toml or ./.aca/config.toml");
    println!("    2. ~/.aca/config.toml");
    println!("    3. /etc/aca/config.toml (Unix) or %PROGRAMDATA%\\aca\\config.toml (Windows)");
    println!("    4. Built-in defaults");
    println!();
    println!("  Use --show-config to see the current configuration discovery status.");
}

/// Display version information
pub fn show_version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_input_determination() {
        // Single task file
        let result =
            Args::determine_task_input(None, Some(PathBuf::from("task.md")), None).unwrap();

        if let TaskInput::SingleFile(path) = result {
            assert_eq!(path, PathBuf::from("task.md"));
        } else {
            panic!("Expected SingleFile");
        }

        // Task list
        let result =
            Args::determine_task_input(None, None, Some(PathBuf::from("tasks.txt"))).unwrap();

        if let TaskInput::TaskList(path) = result {
            assert_eq!(path, PathBuf::from("tasks.txt"));
        } else {
            panic!("Expected TaskList");
        }

        // Config with tasks
        let result =
            Args::determine_task_input(Some(PathBuf::from("config.toml")), None, None).unwrap();

        if let TaskInput::ConfigWithTasks(path) = result {
            assert_eq!(path, PathBuf::from("config.toml"));
        } else {
            panic!("Expected ConfigWithTasks");
        }
    }

    #[test]
    fn test_multiple_inputs_error() {
        // Should error with multiple inputs
        let result = Args::determine_task_input(
            Some(PathBuf::from("config.toml")),
            Some(PathBuf::from("task.md")),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_no_inputs_error() {
        // Should error with no inputs
        let result = Args::determine_task_input(None, None, None);
        assert!(result.is_err());
    }
}
