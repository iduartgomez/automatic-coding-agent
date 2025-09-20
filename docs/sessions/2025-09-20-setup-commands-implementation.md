# Setup Commands and Error Handling Implementation

**Date**: September 20, 2025
**Session Duration**: ~45 minutes
**Status**: ✅ Complete

## Overview

Implemented a comprehensive setup commands system with sophisticated error handling strategies for the automatic coding agent. This allows running shell commands before system initialization with configurable error recovery mechanisms.

## Implemented Features

### 1. Setup Commands Core Types (`src/task/types.rs`)

Added comprehensive types for setup command management:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetupCommand {
    pub id: Uuid,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub required: bool,
    pub error_handler: Option<ErrorHandler>,
}
```

#### Error Handling Strategies

- **Skip Strategy**: Continue despite command failure
- **Retry Strategy**: Retry with configurable attempts and exponential backoff
- **Backup Strategy**: Analyze stdout/stderr and conditionally run backup shell commands

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ErrorStrategy {
    Skip,
    Retry { max_attempts: u32, delay: Duration },
    Backup {
        condition: OutputCondition,
        backup_command: String,
        backup_args: Vec<String>,
    },
}
```

#### Output Analysis

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutputCondition {
    pub check_stdout: bool,
    pub check_stderr: bool,
    pub contains: Option<String>,
    pub not_contains: Option<String>,
    pub exit_code_range: Option<(i32, i32)>,
}
```

### 2. Agent Integration (`src/integration.rs`)

Extended `AgentConfig` to include setup commands:

```rust
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub workspace_path: std::path::PathBuf,
    pub session_config: SessionManagerConfig,
    pub task_config: TaskManagerConfig,
    pub claude_config: ClaudeConfig,
    pub setup_commands: Vec<SetupCommand>, // New field
}
```

Integrated setup execution into `AgentSystem::new()`:

```rust
pub async fn new(config: AgentConfig) -> Result<Self> {
    // Execute setup commands first, before any other initialization
    if !config.setup_commands.is_empty() {
        info!("Executing setup commands before system initialization...");
        Self::execute_setup_commands(&config.setup_commands).await?;
    }
    // ... rest of initialization
}
```

### 3. Shell Command Execution

Implemented robust shell command execution with:

- **Timeout handling** using `tokio::time::timeout`
- **Working directory support**
- **Comprehensive error capture** (stdout, stderr, exit codes)
- **Duration tracking** for performance monitoring
- **Proper async/await patterns**

Key methods added:
- `execute_setup_commands()` - Main orchestrator
- `execute_shell_command()` - Individual command execution
- `handle_command_error()` - Error strategy dispatcher
- `retry_command()` - Retry logic with backoff
- `execute_backup_command()` - Backup command execution
- `should_run_backup()` - Output analysis for backup triggers

### 4. Builder Pattern Support

Added fluent builder methods for easy command construction:

```rust
impl SetupCommand {
    pub fn new(name: &str, command: &str) -> Self { ... }
    pub fn with_args(mut self, args: Vec<String>) -> Self { ... }
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self { ... }
    pub fn with_timeout(mut self, timeout: Duration) -> Self { ... }
    pub fn optional(mut self) -> Self { ... }
    pub fn with_error_handler(mut self, handler: ErrorHandler) -> Self { ... }
}
```

## Testing and Validation

### 1. Comprehensive Test Examples

Created three test examples demonstrating different aspects:

#### `examples/setup_commands_demo.rs`
- ✅ 6 different setup commands with various configurations
- ✅ File creation/cleanup verification
- ✅ Working directory tests
- ✅ Optional command handling
- ✅ Timeout configuration

#### `examples/error_handling_demo.rs`
- ✅ Skip strategy validation
- ✅ Retry strategy with timing verification
- ✅ Backup strategy conditional execution
- ✅ Required command failure prevention
- ✅ Multiple error scenario testing

#### `examples/backup_strategy_test.rs`
- ✅ Conditional backup command execution
- ✅ Output analysis pattern matching
- ✅ Stderr content-based triggers

### 2. Test Results

All tests passed successfully:

```
🚀 Setup Commands Demo: ✅ PASSED (194ms)
🧪 Error Handling Strategies Demo: ✅ PASSED
🔄 Backup Strategy Test: ✅ PASSED
Unit Tests: ✅ 35/35 PASSED
Clippy: ✅ No warnings
```

## Technical Implementation Details

### Async Command Execution

Used `tokio::process::Command` for non-blocking shell execution:

```rust
async fn execute_shell_command(cmd: &SetupCommand) -> Result<SetupResult> {
    let mut command = Command::new(&cmd.command);
    command.args(&cmd.args)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

    if let Some(timeout) = cmd.timeout {
        let timeout_std = timeout.to_std()?;
        tokio::time::timeout(timeout_std, command.output()).await?
    } else {
        command.output().await
    }
}
```

### Error Recovery Logic

Implemented sophisticated error handling with pattern matching:

```rust
match &handler.strategy {
    ErrorStrategy::Skip => {
        warn!("Skipping failed command: {}", cmd.name);
        Ok(())
    }
    ErrorStrategy::Retry { max_attempts, delay } => {
        Self::retry_command(cmd, *max_attempts, *delay).await
    }
    ErrorStrategy::Backup { condition, backup_command, backup_args } => {
        if Self::should_run_backup(result, condition) {
            Self::execute_backup_command(backup_command, backup_args, &cmd.working_dir).await
        } else {
            Err(anyhow::anyhow!("Backup condition not met"))
        }
    }
}
```

### Output Analysis Engine

Built flexible output analysis for backup triggers:

```rust
fn should_run_backup(result: &SetupResult, condition: &OutputCondition) -> bool {
    // Exit code range validation
    if let Some((min, max)) = condition.exit_code_range
        && (result.exit_code < min || result.exit_code > max) {
            return false;
        }

    // Content pattern matching
    let output = if condition.check_stdout { &result.stdout } else { &result.stderr };

    // Required content check
    if let Some(must_contain) = &condition.contains {
        if !output.contains(must_contain) { return false; }
    }

    // Forbidden content check
    if let Some(must_not_contain) = &condition.not_contains {
        if output.contains(must_not_contain) { return false; }
    }

    true
}
```

## Example Usage

```rust
let setup_commands = vec![
    // Simple command
    SetupCommand::new("check_rust", "rustc")
        .with_args(vec!["--version".to_string()]),

    // Optional command with skip strategy
    SetupCommand::new("optional_tool", "which")
        .with_args(vec!["some-tool".to_string()])
        .optional()
        .with_error_handler(ErrorHandler::skip("skip_missing")),

    // Command with retry strategy
    SetupCommand::new("network_check", "ping")
        .with_args(vec!["-c".to_string(), "1".to_string(), "8.8.8.8".to_string()])
        .with_timeout(Duration::seconds(5))
        .with_error_handler(ErrorHandler::retry("retry_ping", 3, Duration::seconds(1))),

    // Command with backup strategy
    SetupCommand::new("check_docker", "docker")
        .with_args(vec!["--version".to_string()])
        .with_error_handler(ErrorHandler::backup(
            "docker_fallback",
            OutputCondition::stderr_contains("command not found"),
            "echo",
            vec!["Using alternative container runtime".to_string()]
        )),
];

let config = AgentConfig {
    setup_commands,
    ..Default::default()
};

// Setup commands execute automatically during initialization
let agent = AgentSystem::new(config).await?;
```

## Architecture Impact

This implementation extends the agent system architecture:

```text
┌─────────────────────────────────────┐
│           AgentSystem               │
│  ┌─────────────────────────────────┐│
│  │     Setup Commands Phase       ││  ← NEW
│  │  ┌─────────────────────────────┐││
│  │  │  Shell Command Executor    │││
│  │  │  Error Recovery Manager    │││
│  │  │  Output Analysis Engine    │││
│  │  └─────────────────────────────┘││
│  └─────────────────────────────────┘│
│  ┌─────────────┐ ┌─────────────┐   │
│  │    Task     │ │   Session   │   │
│  │  Manager    │ │   Manager   │   │
│  └─────────────┘ └─────────────┘   │
└─────────────────────────────────────┘
```

## Files Modified

1. **`src/task/types.rs`** - Added setup command types (470+ lines)
2. **`src/integration.rs`** - Added shell execution logic (200+ lines)
3. **`examples/setup_commands_demo.rs`** - Comprehensive demo (110 lines)
4. **`examples/error_handling_demo.rs`** - Error strategy tests (120 lines)
5. **`examples/backup_strategy_test.rs`** - Backup validation (35 lines)

## Next Steps

The setup commands system is now ready for production use. Future enhancements could include:

- Environment variable substitution in commands
- Command dependency chains
- Parallel execution groups
- Setup command templates/presets
- Integration with package managers (npm, cargo, pip)

## Lessons Learned

1. **Type Safety**: Using strong typing for command configuration prevented runtime errors
2. **Async Patterns**: Proper timeout handling required careful async/await orchestration
3. **Error Recovery**: Sophisticated error handling significantly improves system robustness
4. **Testing Strategy**: Multiple focused test examples caught edge cases early
5. **Builder Pattern**: Fluent API made command configuration intuitive and maintainable