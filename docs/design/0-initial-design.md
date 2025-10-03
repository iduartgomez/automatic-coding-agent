# aca (Automatic Coding Agent) - Design Document

## Overview

A Rust-based agentic tool that automates coding tasks using multiple LLM providers. The system features dynamic task trees, comprehensive session persistence, and full resumability for long-running automated coding sessions. Supports Claude (CLI/API), OpenAI, and local models with intelligent task parsing and execution planning.

## Deliverable Documents

This design has been broken down into focused deliverable documents:

- **[1.1 Architecture Overview](1.1-architecture-overview.md)** - Comprehensive system architecture, dual-mode design, component interfaces, and resource management
- **[1.2 Task Management System](1.2-task-management.md)** - Task tree architecture, scheduling algorithms, and dynamic task management
- **[1.3 Session Persistence System](1.3-session-persistence.md)** - State management, persistence formats, and recovery mechanisms
- **[1.4 Claude Code Integration](1.4-claude-integration.md)** - Claude Code SDK integration, rate limiting, and conversation management
- **[1.5 LLM Provider Abstraction](1.5-llm-provider-abstraction.md)** - Multi-provider support, unified interface, and provider capabilities
- **[1.6 Configuration & Security](1.6-configuration-security.md)** - Configuration management, security controls, and operational monitoring

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        aca System                           │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                   CLI Frontend                          │ │
│  │   - Argument parsing (clap)                            │ │
│  │   - Task file loading (Markdown/TOML)                   │ │
│  │   - Execution mode selection                            │ │
│  │   - Plan dumping and loading                            │ │
│  └─────────────────────────────────────────────────────────┘ │
│                             │                               │
│                             ▼                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │           Intelligent Task Parser (Optional)            │ │
│  │   - LLM-based task analysis                            │ │
│  │   - Dependency detection                                │ │
│  │   - Priority/complexity estimation                      │ │
│  │   - Execution strategy planning                         │ │
│  └─────────────────────────────────────────────────────────┘ │
│                             │                               │
│                             ▼                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Agent Integration Layer                    │ │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐ │ │
│  │  │Task Manager │  │LLM Providers │  │Session Manager │ │ │
│  │  │- Task tree  │  │- Claude CLI  │  │- Checkpoints   │ │ │
│  │  │- Scheduler  │  │- Claude API  │  │- Persistence   │ │ │
│  │  │- Execution  │  │- OpenAI      │  │- Recovery      │ │ │
│  │  │- Progress   │  │- Local (Ollama)│  │- State mgmt  │ │ │
│  │  └─────────────┘  └──────────────┘  └────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                             │
│  Working Directory:                                         │
│  .aca/                 - Session metadata and checkpoints   │
│  .aca/sessions/        - Per-session state                  │
│  logs/                 - Execution logs                     │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. CLI Frontend

**Responsibilities:**

- Parse command-line arguments using clap
- Load task files (Markdown or TOML formats)
- Initialize session state or resume from checkpoint
- Configure LLM provider mode (CLI/API)
- Handle execution plan dumping and loading
- Provide progress monitoring and reporting

**Key Operations:**

- Task file parsing (structured TOML or intelligent Markdown parsing)
- Execution plan analysis and review workflow
- Session checkpoint management
- Provider configuration and selection

### 2. Intelligent Task Parser

**Responsibilities:**

- Analyze unstructured task descriptions using LLM
- Detect hierarchical task relationships
- Identify task dependencies
- Estimate priority and complexity
- Determine optimal execution strategies
- Support markdown file references in task descriptions

### 3. Agent Integration Layer

**Responsibilities:**

- Execute task automation logic
- Interface with configured LLM providers
- Manage dynamic task tree with execution
- Handle provider-specific rate limiting
- Maintain session context and conversation state
- Persist state for resumability

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
.aca/sessions/{session_id}/
├── meta/
│   └── session.json        # Session metadata and task hierarchy
├── claude/                 # Claude Code conversation state
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

## LLM Provider Integration

### Provider Abstraction Layer

The system supports multiple LLM providers through a unified async interface:

```rust
pub trait LlmProvider: Send + Sync {
    fn send_message<'a>(
        &'a mut self,
        prompt: &'a str,
        system_message: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String, LlmError>>;
}
```

### Supported Providers

1. **Claude CLI** (Default)
   - Uses `claude` command-line tool
   - JSON output format for structured responses
   - Subprocess-based execution with output parsing
   - Conversational state persistence

2. **Claude API**
   - Direct Anthropic API integration
   - Message-based conversation history
   - Streaming support (future)
   - Token usage tracking

3. **OpenAI**
   - OpenAI API compatibility
   - GPT-4 and other models
   - Standard chat completion interface

4. **Local Models (Ollama)**
   - Local model execution
   - Privacy-focused option
   - No external API dependencies

### Provider Mode Configuration

- **CLI Mode**: Default, uses subprocess execution
- **API Mode**: Direct API calls with credentials
- Configurable via command-line flags or config file

## Current Implementation Status

### ✅ Implemented Features
- CLI frontend with clap argument parsing
- Intelligent task parser with LLM-based analysis
- Task management system with dynamic tree structure
- Session persistence with checkpoint/resume
- LLM provider abstraction (Claude CLI/API, OpenAI, Ollama)
- Execution plan dumping and loading
- Markdown file reference resolution
- Dependency mapping and detection
- `.aca` directory structure for session state
- TOML configuration support

### 🚧 Planned Features
- Docker containerization for isolated execution
- Headless Claude Code SDK integration via WebSocket
- Multi-container distributed execution
- Advanced rate limiting with usage tracking
- Web dashboard for real-time monitoring
- Plugin system for extensible task handlers

## Intelligent Task Parsing (Implemented)

### LLM-Based Analysis

The intelligent parser uses LLM capabilities to analyze unstructured task descriptions:

**Features:**
- Hierarchical task structure detection
- Automatic dependency identification
- Priority and complexity estimation
- Execution strategy determination (Sequential/Parallel/Intelligent)
- File reference resolution (markdown links to actual files)
- Custom system prompt support via `--append-system-prompt`

### Execution Plan Workflow

1. **Analyze**: `aca --task-file tasks.md --dry-run --dump-plan plan.json`
2. **Review**: Examine and modify `plan.json` as needed
3. **Execute**: `aca --execution-plan plan.json`

This allows for human review and modification before execution.

## Agent Execution Flow

### 1. Initialization Phase

1. Load or create session state from `.aca/sessions/{session_id}`
2. Parse task file (structured TOML or intelligent Markdown parsing)
3. Build task tree from parsed tasks or execution plan
4. Initialize LLM provider (CLI/API mode)
5. Set up session logging and progress tracking

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

### Binary Distribution

```bash
# Build optimized release binary
cargo build --release

# The binary is self-contained with no Docker dependencies
# Requires only: Rust runtime and configured LLM provider (claude CLI or API keys)
```

### Usage Examples

```bash
# Structured TOML task file
aca --task-file tasks.toml

# Intelligent Markdown parsing
aca --task-file tasks.md --use-intelligent-parser \
    --context "project context" \
    --append-system-prompt "additional instructions"

# Execution plan workflow
aca --task-file tasks.md --dry-run --dump-plan plan.json
# Review and edit plan.json
aca --execution-plan plan.json

# Resume from checkpoint
aca resume <checkpoint-id>

# List available checkpoints
aca --list-checkpoints
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

## Future Enhancements (Planned)

### Docker Containerization
- Isolated execution environments
- Read-only repository mounts
- Separate workspace for modifications
- Resource limits (CPU, memory, network)
- Container lifecycle management

### Advanced Features
1. **Multi-model Support**: Support for different Claude models based on task complexity
2. **Distributed Execution**: Run multiple agent containers in parallel
3. **Web Dashboard**: Real-time monitoring and control interface
4. **Plugin System**: Extensible task handlers for specific domains
5. **Integration APIs**: REST/GraphQL APIs for external system integration
6. **Headless Claude Code SDK**: Direct WebSocket integration for better control

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
