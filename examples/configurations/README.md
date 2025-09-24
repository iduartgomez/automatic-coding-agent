# Configuration Examples

This directory contains example configuration files for running ACA in structured configuration mode.

## 📁 Configuration Files (`--config`)

### `default-config.toml`
**Use case:** Standard ACA configuration template

A clean, well-documented configuration showing all available settings with sensible defaults. Perfect for:
- Understanding all configuration options
- Creating custom configurations
- Production deployments

**Command:** `aca --config examples/configurations/default-config.toml`

**Key features:**
- Session management settings (auto-save, checkpoints)
- Task execution configuration (retries, concurrency)
- Claude Code integration settings (rate limits, context)
- Comprehensive error handling configuration

### `simple-tasks.toml` ⚠️
**Use case:** Legacy task configuration format (may be outdated)

This file shows an older approach where tasks were defined directly in the TOML configuration. After the recent ExecutionPlan refactoring, this format may no longer be fully supported.

**Note:** Consider this file for historical reference or migration purposes only.

## 🔧 Configuration Structure

### Core Sections

```toml
workspace_path = "/path/to/your/project"
setup_commands = []  # Commands to run before tasks

[session_config]
# Session management settings

[task_config]
# Task execution behavior

[claude_config]
# Claude Code integration
```

### Session Management
Controls how ACA manages session state and checkpoints:
- `auto_save_interval_minutes`: Automatic state saving frequency
- `auto_checkpoint_interval_minutes`: Automatic checkpoint creation
- `enable_crash_recovery`: Recovery from unexpected shutdowns

### Task Configuration
Controls task execution behavior:
- `auto_retry_failed_tasks`: Retry failed tasks automatically
- `max_concurrent_tasks`: Parallel task execution limit
- `enable_task_metrics`: Performance tracking

### Claude Integration
Configures Claude Code API integration:
- Rate limiting settings
- Context management
- Session configuration
- Usage tracking

## 💡 Configuration Tips

### Setup Commands
Use setup commands to prepare your environment:
```toml
setup_commands = [
    { name = "install_deps", command = "npm install" },
    { name = "build_project", command = "cargo build" }
]
```

### Workspace Path
Set the workspace to your project directory:
```toml
workspace_path = "/Users/you/my-project"
```

### Rate Limiting
Adjust based on your Claude API tier:
```toml
[claude_config.rate_limits]
max_tokens_per_minute = 40000
max_requests_per_minute = 50
```

### Development vs Production
For development, consider:
- More frequent checkpointing
- Verbose logging enabled
- Lower rate limits for safety

For production:
- Optimized checkpoint intervals
- Error recovery enabled
- Monitoring configured