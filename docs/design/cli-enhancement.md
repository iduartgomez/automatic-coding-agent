# CLI Enhancement Design

## Overview

This document outlines the design for enhancing the CLI interface of the automatic coding agent to support both batch and interactive execution modes, with proper argument parsing using lexopt.

## Current State

The current implementation has a basic "Hello, world!" main.rs. The system needs a proper CLI interface that can:
- Load task configurations from files
- Execute in batch mode (default)
- Optionally run in interactive mode
- Parse command-line arguments efficiently

## Design Goals

1. **Batch Mode Default**: The primary execution mode should be batch processing of predefined tasks
2. **Minimal Dependencies**: Use lexopt for argument parsing (lightweight, no additional dependencies)
3. **Configuration-Driven**: Tasks should be loadable from TOML configuration files
4. **Mode Flexibility**: Support both batch and interactive modes
5. **User-Friendly**: Clear help messages and error handling

## CLI Interface Design

### Command Structure

```bash
# Default batch mode - execute tasks from config
cargo run -- --config tasks.toml

# Batch mode with explicit flag
cargo run -- --batch --config tasks.toml

# Interactive mode
cargo run -- --interactive

# Help
cargo run -- --help

# Version
cargo run -- --version
```

### Argument Specification

| Argument | Short | Description | Default | Required |
|----------|-------|-------------|---------|----------|
| `--config` | `-c` | Path to task configuration file | None | Yes (batch mode) |
| `--batch` | `-b` | Run in batch mode | true | No |
| `--interactive` | `-i` | Run in interactive mode | false | No |
| `--workspace` | `-w` | Override workspace path | From config | No |
| `--help` | `-h` | Show help message | - | No |
| `--version` | `-V` | Show version | - | No |
| `--verbose` | `-v` | Verbose output | false | No |
| `--dry-run` | `-n` | Show what would be executed | false | No |

### Execution Modes

#### Batch Mode (Default)

```rust
// Primary execution path
struct BatchConfig {
    config_path: PathBuf,
    workspace_override: Option<PathBuf>,
    verbose: bool,
    dry_run: bool,
}
```

**Behavior:**
1. Load task configuration from specified TOML file
2. Initialize AgentSystem with loaded configuration
3. Execute all tasks in the configuration
4. Report completion status and exit

**Configuration File Format:**
```toml
# Task configuration extends AgentConfig
workspace_path = "/path/to/workspace"

[[tasks]]
name = "setup_environment"
description = "Initialize development environment"
commands = ["git clone repo", "npm install"]

[[tasks]]
name = "run_tests"
description = "Execute test suite"
commands = ["npm test"]
depends_on = ["setup_environment"]

[session_config]
auto_save_interval_minutes = 5

[task_config]
max_concurrent_tasks = 4
```

#### Interactive Mode

```rust
// Secondary execution path
struct InteractiveConfig {
    workspace: Option<PathBuf>,
    verbose: bool,
}
```

**Behavior:**
1. Start interactive prompt
2. Allow user to define tasks dynamically
3. Execute tasks as they are defined
4. Maintain session state between commands

## Implementation Architecture

### Main Function Flow

```rust
fn main() -> Result<()> {
    let args = Args::parse()?;

    match args.mode {
        ExecutionMode::Batch(config) => run_batch_mode(config).await,
        ExecutionMode::Interactive(config) => run_interactive_mode(config).await,
        ExecutionMode::Help => show_help(),
        ExecutionMode::Version => show_version(),
    }
}
```

### Argument Parsing with lexopt

```rust
use lexopt::prelude::*;

#[derive(Debug)]
enum ExecutionMode {
    Batch(BatchConfig),
    Interactive(InteractiveConfig),
    Help,
    Version,
}

#[derive(Debug)]
struct Args {
    mode: ExecutionMode,
}

impl Args {
    fn parse() -> Result<Self> {
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
                    return Ok(Args { mode: ExecutionMode::Help });
                }
                Short('V') | Long("version") => {
                    return Ok(Args { mode: ExecutionMode::Version });
                }
                _ => return Err(arg.unexpected()),
            }
        }

        let mode = if force_interactive {
            ExecutionMode::Interactive(InteractiveConfig { workspace, verbose })
        } else {
            let config_path = config_path.ok_or("--config is required for batch mode")?;
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
```

### Task Configuration Loading

```rust
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
    agent_config: AgentConfig,
}

impl TaskFileConfig {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: TaskFileConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn to_agent_config(self) -> AgentConfig {
        AgentConfig {
            workspace_path: self.workspace_path,
            setup_commands: self.tasks.into_iter()
                .map(|task| task.to_setup_command())
                .collect(),
            ..self.agent_config
        }
    }
}
```

## Error Handling Strategy

### Graceful Degradation
1. **Invalid Arguments**: Show help message and exit cleanly
2. **Missing Config File**: Clear error message with suggested fix
3. **Invalid Config**: Detailed parsing error with line numbers
4. **Task Execution Errors**: Continue with other tasks unless critical

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("Configuration file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Task execution failed: {task_name}")]
    TaskFailed { task_name: String },

    #[error("Workspace not accessible: {path}")]
    WorkspaceError { path: PathBuf },
}
```

## Integration Points

### With Existing AgentConfig
- The CLI will load TaskFileConfig and convert to AgentConfig
- Existing TOML serialization methods will be reused
- Workspace path can be overridden via CLI argument

### With Task Management System
- Tasks from config file will be converted to SetupCommand instances
- Dependency resolution will be handled by existing task manager
- Error handling strategies will be preserved

### With Session Persistence
- Batch mode will create temporary sessions
- Interactive mode will maintain persistent sessions
- Session data will be saved to `/session` directory

## Testing Strategy

### Unit Tests
- Argument parsing edge cases
- Configuration file loading
- Error handling scenarios

### Integration Tests
- End-to-end batch execution
- Interactive mode workflows
- Configuration validation

### Example Configurations
```toml
# examples/simple-task.toml
workspace_path = "/tmp/test-workspace"

[[tasks]]
name = "hello_world"
description = "Simple hello world task"
commands = ["echo 'Hello, World!'"]

[session_config]
auto_save_interval_minutes = 1

[task_config]
max_concurrent_tasks = 1
```

## Implementation Phases

### Phase 1: Basic CLI Structure
1. Add lexopt dependency
2. Implement argument parsing
3. Create basic execution modes
4. Add help and version commands

### Phase 2: Batch Mode Implementation
1. Implement TaskFileConfig loading
2. Convert TaskConfig to SetupCommand
3. Integrate with existing AgentSystem
4. Add error handling and reporting

### Phase 3: Interactive Mode
1. Create interactive prompt
2. Implement dynamic task creation
3. Add session persistence
4. Support real-time task execution

### Phase 4: Advanced Features
1. Add task dependency visualization
2. Implement progress reporting
3. Add configuration validation
4. Support configuration templates

## Dependencies

### New Dependencies
- `lexopt = "0.3"` - Minimal argument parser
- `thiserror = "1.0"` - Error handling (if not already present)

### Existing Dependencies (Reused)
- `toml = "0.8"` - Configuration parsing
- `tokio` - Async runtime
- `serde` - Serialization

## Backward Compatibility

The current "Hello, world!" main.rs will be completely replaced, but this is acceptable since:
1. The project is in early development stage
2. No existing users depend on current CLI
3. The new interface provides significantly more value

## Future Considerations

### Configuration Management
- Support for configuration file discovery (search up directory tree)
- Environment variable support for common settings
- Configuration validation and schema documentation

### Advanced Task Features
- Task templating and parameterization
- Conditional task execution
- Dynamic task generation based on environment

### Integration Enhancements
- IDE integration for task development
- CI/CD pipeline integration
- Remote task execution support