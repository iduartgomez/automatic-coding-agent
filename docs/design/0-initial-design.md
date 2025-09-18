# Claude Code Agent - Design Document

## Overview

A Rust-based agentic tool that automates coding tasks using Claude Code in headless mode. The system operates in two distinct modes: a host-side session initializer and an in-container agent that executes tasks using a dynamic task tree with full persistence and resumability.

## Deliverable Documents

This design has been broken down into focused deliverable documents:

- **[1.1 Architecture Overview](1.1-architecture-overview.md)** - Comprehensive system architecture, dual-mode design, component interfaces, and resource management
- **[1.2 Task Management System](1.2-task-management.md)** - Task tree architecture, scheduling algorithms, and dynamic task management
- **[1.3 Session Persistence System](1.3-session-persistence.md)** - State management, persistence formats, and recovery mechanisms
- **[1.4 Claude Code Integration](1.4-claude-integration.md)** - Claude Code SDK integration, rate limiting, and conversation management
- **[1.5 Docker Deployment System](1.5-docker-deployment.md)** - Container orchestration, volume management, and deployment strategies
- **[1.6 Configuration & Security](1.6-configuration-security.md)** - Configuration management, security controls, and operational monitoring

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        Host System                          │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │   CLI Frontend  │    │     Session Manager             │ │
│  │   - Parse args  │────│   - Docker lifecycle           │ │
│  │   - Task input  │    │   - Volume management          │ │
│  │   - Config      │    │   - State persistence          │ │
│  └─────────────────┘    └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                                    │
                                    │ Docker API
                                    ▼
┌─────────────────────────────────────────────────────────────┐
│                    Docker Container                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                 Agent Runtime                           │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │ │
│  │  │Task Manager │  │Claude Code  │  │Session Context  │ │ │
│  │  │- Task tree  │  │Interface    │  │- Conversation   │ │ │
│  │  │- Scheduler  │  │- Headless   │  │- File changes   │ │ │
│  │  │- State mgmt │  │- Rate limit │  │- Build state    │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                             │
│  Volume Mounts:                                             │
│  /repos     (RO) - Source repositories                     │
│  /workspace (RW) - Working directory                       │
│  /session   (RW) - Persistent session data                 │
│  /logs      (RW) - Session logs and outputs                │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. CLI Frontend & Session Manager (Host)

**Responsibilities:**

- Parse command-line arguments and configuration
- Initialize Docker environment with proper volume mounts
- Manage container lifecycle (start, stop, cleanup)
- Handle session persistence and resumption
- Provide status monitoring and progress reporting

**Key Operations:**

- `init` - Initialize new session with task list and repository references
- `resume` - Resume existing session from checkpoint
- `status` - Query current session state
- `cleanup` - Clean up Docker resources

### 2. Agent Runtime (Container)

**Responsibilities:**

- Execute the actual task automation logic
- Interface with Claude Code in headless mode
- Manage dynamic task tree with subtask creation
- Handle rate limiting and error recovery
- Maintain session context and logging

## Task Management System

### Task Tree Structure

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub parent_id: Option<TaskId>,
    pub children: Vec<TaskId>,
    pub dependencies: Vec<TaskId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: TaskMetadata,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked(String),
    Completed,
    Failed(String),
    Skipped(String),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TaskMetadata {
    pub priority: u8,
    pub estimated_complexity: Option<u8>,
    pub repository_refs: Vec<String>,
    pub file_refs: Vec<PathBuf>,
    pub tags: Vec<String>,
}
```

### Task Tree Operations

- **Dynamic Subtask Creation**: Agent can break down complex tasks into smaller subtasks
- **Dependency Resolution**: Automatic handling of task dependencies
- **Context Inheritance**: Subtasks inherit relevant context from parent tasks
- **Progress Tracking**: Real-time status updates throughout the tree

## Session Persistence

### State Components

1. **Task Tree State**

   - Complete task hierarchy with status
   - Inter-task dependencies and relationships
   - Progress metrics and timing data

2. **Claude Code Context**

   - Conversation history and context
   - Session configuration and preferences
   - Model usage and rate limiting state

3. **File System State**

   - Modified files and their change history
   - Build artifacts and compilation results
   - Workspace directory structure

4. **Execution Logs**
   - Structured task execution logs
   - Claude Code interaction traces
   - Error logs and debugging information

### Persistence Format

```
/session/
├── task_tree.json          # Complete task hierarchy
├── claude_context/         # Claude Code conversation state
│   ├── messages.json
│   ├── session_config.json
│   └── rate_limit_state.json
├── file_changes/          # File modification tracking
│   ├── change_log.json
│   └── snapshots/
└── execution_logs/        # Structured execution logs
    ├── task_logs/
    └── system_logs/
```

## Claude Code Integration

### Headless Mode Interface

**Key Reference**: [Claude Code SDK Headless Documentation](https://docs.claude.com/en/docs/claude-code/sdk/sdk-headless)

The headless SDK provides programmatic control over Claude Code sessions through WebSocket connections and structured message passing.

```rust
pub struct ClaudeCodeInterface {
    session: HeadlessSession,
    rate_limiter: RateLimiter,
    context_manager: ContextManager,
}

impl ClaudeCodeInterface {
    pub async fn execute_task(&mut self, task: &Task) -> Result<TaskResult>;
    pub async fn ask_question(&mut self, question: &str) -> Result<String>;
    pub async fn request_file_analysis(&mut self, files: &[PathBuf]) -> Result<AnalysisResult>;
    pub async fn create_subtasks(&mut self, context: &str) -> Result<Vec<Task>>;
}
```

### Rate Limiting Strategy

**Key Reference**: [ccusage Session Blocks Implementation](https://raw.githubusercontent.com/ryoppippi/ccusage/refs/heads/main/apps/ccusage/src/_session-blocks.ts)

The ccusage project demonstrates effective rate limiting patterns for Claude Code API usage:

- **Token-aware limiting**: Track token usage per model (see ccusage token tracking logic)
- **Adaptive backoff**: Exponential backoff with jitter (reference ccusage retry mechanisms)
- **Request queuing**: Queue requests during rate limit periods
- **Usage reporting**: Real-time usage metrics and projections

**Implementation Notes from ccusage**:

- Session blocking based on usage thresholds
- Dynamic rate adjustment based on API responses
- Persistent usage tracking across sessions

## Docker Environment

### Container Configuration

```dockerfile
FROM rust:1.75-slim
# Install Claude Code CLI and dependencies
# Set up working environment
WORKDIR /agent
COPY agent-binary /agent/
ENTRYPOINT ["/agent/agent-binary"]
```

### Volume Mounting Strategy

```bash
docker run \
  -v "$REPOS_DIR:/repos:ro" \
  -v "$WORKSPACE_DIR:/workspace:rw" \
  -v "$SESSION_DIR:/session:rw" \
  -v "$LOGS_DIR:/logs:rw" \
  claude-code-agent:latest
```

## Agent Execution Flow

### 1. Initialization Phase

1. Load or create session state from `/session`
2. Parse initial task list and build task tree
3. Initialize Claude Code headless session
4. Set up rate limiting and logging infrastructure

### 2. Task Execution Loop

```
while has_pending_tasks() {
    task = select_next_task()

    match execute_task(task) {
        Success(result) => {
            update_task_status(task, Completed)
            process_result(result)
            create_subtasks_if_needed(result)
        }
        Failure(error) => {
            update_task_status(task, Failed)
            handle_error(error)
            maybe_create_retry_task(task)
        }
        Blocked(reason) => {
            update_task_status(task, Blocked)
            schedule_retry_or_skip(task)
        }
    }

    persist_session_state()

    if should_checkpoint() {
        create_checkpoint()
    }
}
```

### 3. Task Selection Algorithm

- **Priority-based**: Higher priority tasks selected first
- **Dependency-aware**: Only select tasks whose dependencies are met
- **Context-optimized**: Prefer tasks that share context with recent work
- **Load-balanced**: Distribute work across different repositories/areas

## Error Handling & Recovery

### Error Categories

1. **Transient Errors**: Network issues, rate limits, temporary API failures
2. **Task Errors**: Code compilation failures, test failures, logical errors
3. **System Errors**: File system issues, Docker problems, resource exhaustion

### Recovery Strategies

- **Automatic Retry**: Exponential backoff for transient errors
- **Task Decomposition**: Break down failed tasks into smaller subtasks
- **Context Reset**: Clear and rebuild context when corrupted
- **Manual Intervention**: Flag tasks requiring human input

## Configuration Management

### Agent Configuration

```toml
[session]
max_duration_hours = 8
checkpoint_interval_minutes = 30
max_concurrent_tasks = 3

[claude_code]
model = "claude-sonnet-4-20250514"
max_tokens = 4000
temperature = 0.1

[rate_limiting]
requests_per_minute = 30
tokens_per_minute = 100000
burst_allowance = 5

[docker]
image = "claude-code-agent:latest"
cpu_limit = "2.0"
memory_limit = "8g"
network_mode = "bridge"

[logging]
level = "info"
structured = true
include_claude_traces = true
```

## Performance Considerations

### Optimization Strategies

1. **Context Reuse**: Maintain Claude Code session across related tasks
2. **Batching**: Group related file operations and API calls
3. **Caching**: Cache compilation results and analysis outputs
4. **Resource Management**: Monitor and limit container resource usage

### Monitoring Metrics

- Task completion rate and average time
- Claude Code API usage and costs
- Container resource utilization
- Error rates by category
- Session checkpoint and resume success rates

## Security Considerations

### Container Security

- **Read-only repositories**: Prevent accidental source code modification
- **Isolated workspace**: Sandbox for all build and test operations
- **Resource limits**: Prevent resource exhaustion attacks
- **Network restrictions**: Limit container network access

### Data Security

- **Session encryption**: Encrypt sensitive session data at rest
- **Access logging**: Audit all file system and API access
- **Credential management**: Secure handling of API keys and tokens

## Deployment & Distribution

### Single Binary Distribution

```bash
# Build optimized release binary
cargo build --release --target x86_64-unknown-linux-musl

# Package with Docker image dependencies
./package.sh # Creates self-contained executable with embedded Docker image
```

### Usage Examples

```bash
# Initialize new session
claude-code-agent init \
  --tasks tasks.json \
  --repos /path/to/repos \
  --workspace /path/to/workspace

# Resume existing session
claude-code-agent resume --session-id abc123

# Monitor progress
claude-code-agent status --session-id abc123

# Clean up
claude-code-agent cleanup --session-id abc123
```

## Implementation References

This section consolidates key external references and examples that inform the implementation of specific components.

### Core References

| Component             | Reference                                                                                                    | Purpose                                          |
| --------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------ |
| **Headless SDK**      | [Claude Code SDK Documentation](https://docs.claude.com/en/docs/claude-code/sdk/sdk-headless)                | Primary API for programmatic Claude Code control |
| **Rate Limiting**     | [ccusage Session Blocks](https://github.com/ryoppippi/ccusage/blob/main/apps/ccusage/src/_session-blocks.ts) | Production-tested rate limiting patterns         |
| **Project Structure** | [ccusage Repository](https://github.com/ryoppippi/ccusage/tree/main)                                         | Overall architecture and organization patterns   |

### Component-Specific Implementation Guides

#### 1. Claude Code Integration

- **SDK Initialization**: Reference headless SDK docs for WebSocket connection setup
- **Message Protocol**: Use SDK documentation for structured request/response formats
- **Session Management**: Follow SDK patterns for maintaining persistent sessions

#### 2. Rate Limiting Implementation

From the ccusage codebase, key patterns to adapt:

```typescript
// Reference pattern from ccusage/_session-blocks.ts
interface SessionBlock {
  startTime: number;
  endTime: number;
  tokenUsage: number;
  requestCount: number;
}
```

- Session-based usage tracking
- Proactive blocking before hitting limits
- Exponential backoff with configurable multipliers
- Usage projection and early warning systems

#### 3. Docker Container Management

- **Volume Mounting**: Use Docker API best practices for read-only repository mounts
- **Resource Limits**: Reference Docker documentation for CPU/memory constraints
- **Network Isolation**: Implement restricted networking for security

#### 4. Session Persistence

- **State Serialization**: Use Rust serde patterns for JSON-based persistence
- **Atomic Updates**: Implement write-ahead logging for session state changes
- **Recovery Logic**: Design idempotent operations for crash recovery

### Configuration References

#### Docker Configuration

```yaml
# Reference Docker Compose patterns for volume mounting
services:
  agent:
    volumes:
      - "${REPOS_PATH}:/repos:ro"
      - "${WORKSPACE_PATH}:/workspace:rw"
      - "${SESSION_PATH}:/session:rw"
    deploy:
      resources:
        limits:
          cpus: "2.0"
          memory: 8G
```

#### Rate Limiting Configuration

Based on ccusage patterns:

```toml
[rate_limiting]
# Adapt from ccusage session management
session_duration_minutes = 60
max_tokens_per_session = 100000
max_requests_per_minute = 30
backoff_multiplier = 2.0
max_backoff_seconds = 300
```

### Planned Features

1. **Multi-model Support**: Support for different Claude models based on task complexity
2. **Distributed Execution**: Run multiple agent containers in parallel
3. **Web Dashboard**: Real-time monitoring and control interface
4. **Plugin System**: Extensible task handlers for specific domains
5. **Integration APIs**: REST/GraphQL APIs for external system integration

### Scalability Considerations

- **Horizontal scaling**: Multiple agent containers with shared session state
- **Resource optimization**: Dynamic container sizing based on workload
- **State sharding**: Distribute large session states across multiple storage backends

---

This design provides a robust foundation for building a sophisticated agentic coding assistant that can handle complex, long-running development tasks with full persistence and resumability.

---

## Implementation Roadmap

The implementation should proceed through the following phases:

### Phase 1: Core Foundation (Deliverable 1.1)
- Basic CLI and configuration system
- Docker container management
- Session initialization and cleanup

### Phase 2: Task Management (Deliverable 1.2)
- Task tree data structure and operations
- Basic task scheduling and execution
- Task persistence and state management

### Phase 3: Session Persistence (Deliverable 1.3)
- Comprehensive state persistence system
- Checkpoint and recovery mechanisms
- Data integrity and validation

### Phase 4: Claude Integration (Deliverable 1.4)
- Headless SDK integration
- Rate limiting and usage management
- Context optimization and error recovery

### Phase 5: Production Deployment (Deliverable 1.5)
- Advanced container orchestration
- Volume management and security
- Resource monitoring and optimization

### Phase 6: Security & Operations (Deliverable 1.6)
- Security controls and compliance
- Monitoring and alerting systems
- Performance optimization and analytics

Each phase builds upon the previous ones, allowing for incremental development and testing while maintaining a clear path toward the complete system.
