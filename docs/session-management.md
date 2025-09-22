# Session Management and Persistence

The automatic-coding-agent features a sophisticated session management system that provides comprehensive state persistence, automatic recovery, and checkpoint-based resumability. This document explains how the session system works and the purpose of the various session files.

## Overview

The session management system ensures that all work is continuously preserved and can be resumed at any point, even after unexpected interruptions. The system operates on three core principles:

1. **Continuous Persistence**: All state changes are immediately saved
2. **Atomic Operations**: All changes are transactional with rollback capabilities
3. **Intelligent Recovery**: Automatic detection and recovery from various failure modes

## Core Session Files

### `session.json` - Primary Session State

The `session.json` file is the heart of the session management system. It contains the complete state of the agent session, including:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "metadata": {
    "workspace_path": "/path/to/workspace",
    "created_at": "2025-09-22T10:30:00Z",
    "last_updated": "2025-09-22T11:45:00Z",
    "agent_version": "0.1.0",
    "total_tasks": 5,
    "completed_tasks": 3,
    "failed_tasks": 0
  },
  "task_tree": {
    "nodes": [...],
    "relationships": [...],
    "execution_order": [...]
  },
  "execution_state": {
    "current_task_id": "task-123",
    "phase": "processing",
    "last_checkpoint": "checkpoint-456"
  },
  "context": {
    "environment_vars": {...},
    "working_directory": "/workspace",
    "session_config": {...}
  }
}
```

#### Key Components:

**Metadata**: Session identification, timing, and high-level statistics
- `id`: Unique session identifier (UUID)
- `workspace_path`: Absolute path to the workspace directory
- `created_at`/`last_updated`: Session lifecycle timestamps
- Task counters and status summary

**Task Tree**: Complete hierarchical task structure
- `nodes`: All tasks with their full state and metadata
- `relationships`: Parent-child and dependency relationships
- `execution_order`: Dynamically calculated task execution sequence

**Execution State**: Current processing state
- `current_task_id`: Task currently being processed
- `phase`: Current execution phase (planning, processing, completed, etc.)
- `last_checkpoint`: Reference to the most recent checkpoint

**Context**: Environment and configuration state
- Session-specific environment variables
- Working directory and file system state
- Configuration settings and preferences

### Checkpoints - Recovery Points

Checkpoints are snapshot files that capture the complete system state at specific points in time. They serve as recovery points that allow the system to roll back to a known good state.

#### Checkpoint File Structure

Checkpoints are stored as:
```
.session/checkpoints/checkpoint-{uuid}-{timestamp}.json
```

Example: `checkpoint-7f3a8b2c-1699123456789.json`

#### Checkpoint Contents

```json
{
  "checkpoint_id": "7f3a8b2c-4d6e-8f9a-b1c2-d3e4f5a6b7c8",
  "created_at": "2025-09-22T11:30:00Z",
  "session_state": {
    // Complete session.json state at checkpoint time
  },
  "file_system_state": {
    "tracked_files": [
      {
        "path": "src/main.py",
        "hash": "sha256:abc123...",
        "modified_at": "2025-09-22T11:29:00Z",
        "size": 1024
      }
    ],
    "workspace_snapshot": "snapshot-id-123"
  },
  "claude_context": {
    "conversation_history": [...],
    "context_windows": [...],
    "message_count": 15
  },
  "system_state": {
    "memory_usage": 128,
    "active_processes": [],
    "environment_state": {...}
  }
}
```

#### When Checkpoints Are Created

1. **Automatic Intervals**: Every 30 minutes during active processing
2. **Task Boundaries**: Before starting major tasks or subtask groups
3. **Manual Triggers**: When explicitly requested via API
4. **Error Recovery**: Before attempting recovery operations
5. **System Events**: Before significant state changes

### `.session/` Directory Structure

The complete session directory structure:

```
.session/
├── session.json                 # Primary session state
├── session.lock                 # Session lock file
├── checkpoints/                 # Checkpoint storage
│   ├── checkpoint-001-{uuid}.json
│   ├── checkpoint-002-{uuid}.json
│   └── latest -> checkpoint-002-{uuid}.json
├── logs/                        # Session logs
│   ├── session.log              # Main session log
│   ├── task-execution.log       # Task-specific logs
│   └── claude-subprocess-{id}.log # Claude Code subprocess logs
├── cache/                       # Temporary cache files
│   ├── task-cache/              # Task result cache
│   └── context-cache/           # Claude context cache
├── backups/                     # Session backups
│   └── session-{timestamp}.json.bak
└── recovery/                    # Recovery metadata
    ├── corruption-detection.json
    └── last-known-good.json
```

## Session Lifecycle

### Session Initialization

1. **New Session**: Creates fresh session.json with initial metadata
2. **Session Discovery**: Automatically detects existing sessions in workspace
3. **Recovery Check**: Validates session integrity and attempts recovery if needed
4. **Context Restoration**: Restores Claude context and task state

### Continuous Operation

1. **Auto-Save**: Session state saved every 5 minutes during operation
2. **State Tracking**: All task state changes immediately persisted
3. **Progress Updates**: Real-time updates to task progress and statistics
4. **Context Management**: Claude conversation context continuously synchronized

### Checkpoint Creation Process

1. **State Validation**: Verify current session state integrity
2. **File System Snapshot**: Capture current workspace file state
3. **Context Capture**: Save complete Claude conversation context
4. **Atomic Write**: Write checkpoint file atomically
5. **Reference Update**: Update session.json with new checkpoint reference
6. **Cleanup**: Remove old checkpoints based on retention policy

### Session Recovery

#### Automatic Recovery Scenarios

1. **Process Interruption**: Agent process killed or crashed
2. **System Restart**: Machine reboot or shutdown
3. **Network Issues**: Temporary connectivity problems
4. **Memory Issues**: Out-of-memory or resource exhaustion

#### Recovery Process

1. **Session Detection**: Scan workspace for existing session files
2. **Integrity Check**: Validate session.json and checkpoint integrity
3. **Corruption Detection**: Identify and flag corrupted state
4. **Recovery Strategy**: Choose best recovery approach:
   - **Clean Recovery**: Session intact, resume from current state
   - **Checkpoint Recovery**: Roll back to last valid checkpoint
   - **Partial Recovery**: Reconstruct state from available data
   - **Manual Recovery**: Require user intervention

#### Recovery Strategies

**Level 1 - Clean Resume**:
- Session.json is valid and current
- Continue from last known state
- No data loss

**Level 2 - Checkpoint Rollback**:
- Session.json corrupted but checkpoints available
- Roll back to most recent valid checkpoint
- Minimal data loss (only since last checkpoint)

**Level 3 - Reconstructive Recovery**:
- Multiple corruption issues detected
- Reconstruct session from task logs and file changes
- Some task state may be lost but work preserved

**Level 4 - Manual Recovery**:
- Significant corruption requiring user intervention
- Present recovery options to user
- Allow selective state restoration

## Session Configuration

### Auto-Save Settings

```json
{
  "auto_save": {
    "interval_minutes": 5,
    "on_task_completion": true,
    "on_task_failure": true,
    "on_context_change": true
  }
}
```

### Checkpoint Settings

```json
{
  "checkpoints": {
    "auto_create_interval_minutes": 30,
    "max_checkpoints": 10,
    "retention_hours": 168,
    "compression": true,
    "include_file_snapshots": true
  }
}
```

### Recovery Settings

```json
{
  "recovery": {
    "auto_recovery": true,
    "max_recovery_attempts": 3,
    "recovery_timeout_seconds": 300,
    "backup_before_recovery": true
  }
}
```

## Monitoring and Debugging

### Session Health Monitoring

The session manager continuously monitors:

- **File System**: Workspace file changes and conflicts
- **Memory Usage**: Session state size and memory consumption
- **Lock Status**: Session lock file status and conflicts
- **Integrity**: Session and checkpoint file integrity
- **Performance**: Save/load operation performance

### Debug Information

Session debug information available via:

```bash
# Show session status
automatic-coding-agent --show-session-status

# Validate session integrity
automatic-coding-agent --validate-session

# List available checkpoints
automatic-coding-agent --list-checkpoints

# Recover from specific checkpoint
automatic-coding-agent --recover-from-checkpoint <checkpoint-id>
```

### Log Files

**session.log**: Complete session activity log
```
[2025-09-22 11:30:00] Session started: 550e8400-e29b-41d4-a716-446655440000
[2025-09-22 11:30:05] Task created: implement_auth_system
[2025-09-22 11:30:10] Checkpoint created: checkpoint-001
[2025-09-22 11:35:00] Auto-save completed (5.2ms)
```

**claude-subprocess-{id}.log**: Claude Code subprocess execution logs
```
[2025-09-22 11:30:15] Executing Claude Code command: claude --print --model sonnet -- "Create authentication module"
[2025-09-22 11:30:20] Command completed in 4.8s | Exit code: 0 | Stdout: 1024 bytes
[2025-09-22 11:30:21] Task completed successfully | Input tokens: 245 | Output tokens: 512
```

## Best Practices

### For Users

1. **Regular Checkpoints**: Create manual checkpoints before major changes
2. **Clean Workspaces**: Keep workspace directories organized and clean
3. **Monitor Logs**: Check session logs for errors or warnings
4. **Backup Sessions**: Periodically backup important session files

### For Developers

1. **Session Validation**: Always validate session state before operations
2. **Atomic Operations**: Use transaction-like operations for state changes
3. **Error Handling**: Implement comprehensive error handling with recovery
4. **Performance**: Monitor session file sizes and performance metrics

## Common Issues and Solutions

### Issue: Session Lock Conflicts

**Symptoms**: "Session locked by another process" error
**Solutions**:
1. Check for running agent processes: `ps aux | grep automatic-coding-agent`
2. Remove stale lock file: `rm .session/session.lock`
3. Force unlock: `automatic-coding-agent --force-unlock`

### Issue: Checkpoint Corruption

**Symptoms**: Recovery fails with checkpoint validation errors
**Solutions**:
1. List checkpoints: `automatic-coding-agent --list-checkpoints`
2. Validate specific checkpoint: `automatic-coding-agent --validate-checkpoint <id>`
3. Recover from earlier checkpoint: `automatic-coding-agent --recover-from-checkpoint <id>`

### Issue: Large Session Files

**Symptoms**: Slow session save/load operations
**Solutions**:
1. Enable checkpoint compression in configuration
2. Clean up old task data and logs
3. Implement checkpoint rotation policy
4. Use file system snapshots for large workspaces

### Issue: Context Loss

**Symptoms**: Claude loses conversation context between sessions
**Solutions**:
1. Check context cache: `.session/cache/context-cache/`
2. Verify context synchronization settings
3. Force context reconstruction from logs
4. Review context retention policies

## Advanced Features

### Session Branching

Create alternate session branches for experimental work:

```bash
# Create branch from current session
automatic-coding-agent --create-branch experimental

# List session branches
automatic-coding-agent --list-branches

# Switch to branch
automatic-coding-agent --switch-branch experimental

# Merge branch back to main
automatic-coding-agent --merge-branch experimental
```

### Session Templates

Save and reuse session configurations:

```bash
# Save current session as template
automatic-coding-agent --save-template web-development

# Create new session from template
automatic-coding-agent --from-template web-development
```

### Collaborative Sessions

Share sessions across team members:

```bash
# Export session for sharing
automatic-coding-agent --export-session shared-session.tar.gz

# Import shared session
automatic-coding-agent --import-session shared-session.tar.gz
```

This comprehensive session management system ensures that all work is preserved, recoverable, and can be seamlessly resumed, making the automatic-coding-agent a reliable tool for long-running and complex coding automation tasks.