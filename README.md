# Automatic Coding Agent

[![CI](https://github.com/iduartgomez/automatic-coding-agent/workflows/CI/badge.svg)](https://github.com/iduartgomez/automatic-coding-agent/actions?query=workflow%3ACI)
[![Security Audit](https://github.com/iduartgomez/automatic-coding-agent/workflows/Security%20Audit/badge.svg)](https://github.com/iduartgomez/automatic-coding-agent/actions?query=workflow%3A%22Security+Audit%22)
[![Crates.io](https://img.shields.io/crates/v/aca.svg)](https://crates.io/crates/aca)
[![Documentation](https://docs.rs/aca/badge.svg)](https://docs.rs/aca)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust-based agentic tool that automates coding tasks using multiple LLM providers. The system operates with dynamic task trees, comprehensive session persistence, and full resumability for long-running automated coding sessions. Features a provider-agnostic LLM interface supporting Claude Code, OpenAI Codex, and local models with both CLI and library interfaces.

## Features

### 🎯 **Task Management System**
- **Dynamic Task Tree**: Hierarchical task management with parent-child relationships
- **Intelligent Scheduling**: Multi-factor scoring system with resource-aware prioritization
- **Dependency Resolution**: Complex dependency tracking with circular dependency detection
- **Progress Tracking**: Real-time statistics and completion estimation
- **Auto-Deduplication**: Automatic detection and merging of similar tasks

### 💾 **Session Persistence**
- **Atomic Operations**: Thread-safe persistence with transaction support and rollback
- **Checkpoint System**: UUID-based checkpoint creation with automatic cleanup
- **Recovery Manager**: Intelligent recovery from corruption and failures
- **State Validation**: Comprehensive integrity checking with auto-correction
- **Version Management**: Backward compatibility and migration support

### 🏗️ **Architecture**
- **Modular Design**: Clean separation of concerns with well-defined interfaces
- **Async/Await**: Fully async implementation for non-blocking operations
- **Thread-Safe**: Concurrent operations using Arc/RwLock patterns
- **Event-Driven**: Comprehensive event system for monitoring and automation
- **Resource Management**: Memory, CPU, and storage constraint enforcement

### 🤖 **LLM Provider Abstraction**
- **Multi-Provider Support**: Claude Code, OpenAI Codex, Anthropic API, local models (Ollama)
- **Provider-Agnostic Interface**: Unified API across all LLM providers
- **Automatic Fallback**: Seamless fallback between providers for reliability
- **Rate Limiting**: Provider-specific rate limiting and cost optimization
- **Capability Detection**: Automatic detection of provider features (streaming, function calling, etc.)

#### Codex CLI Provider
- Relies on the `codex` command-line tool (install from [OpenAI Codex](https://github.com/openai/codex)).
- Authenticates through the same login flow as the CLI (e.g., ChatGPT Plus/Pro sign-in) and reuses `~/.codex/config.toml`; no API key required.
- Supports configurable CLI path, profile selection, and smart rate limiting/logging via `ProviderConfig.additional_config`.

### 🧠 **Intelligent Task Parsing**
- **LLM-Based Analysis**: Semantic understanding of task structures using Claude
- **Hierarchical Detection**: Automatic parent-child relationship identification
- **Dependency Analysis**: Smart detection of task dependencies
- **Priority & Complexity**: Context-aware priority and complexity estimation
- **Execution Strategies**: Optimal Sequential/Parallel/Intelligent execution planning
- **Plan Persistence**: Dump, review, modify, and execute execution plans

## Quick Start

### Prerequisites
- Rust 1.90.0 or later
- Cargo

### Installation

```bash
# Clone the repository
git clone https://github.com/iduartgomez/automatic-coding-agent.git
cd automatic-coding-agent

# Build the project
cargo build --release

# Run tests
cargo test

# Run the application
cargo run
```

### Usage

#### CLI Interface

```bash
# Show help (default when no command specified)
aca

# Run intelligent parser with a specific provider
aca --provider claude-code run --use-intelligent-parser tasks.md
aca --provider openai-codex --model gpt-5 run --use-intelligent-parser --dry-run main-tasks.md

# Execute a file (auto-detects type based on extension)
aca run tasks.md           # Markdown task list
aca run config.toml        # TOML config with tasks
aca run plan.json          # JSON execution plan

# Intelligent task parsing with context
aca run .claude/tasks.md --use-intelligent-parser \
    --context "full-stack app" \
    --context "6 month timeline"

# Analyze, review, and execute workflow
aca run tasks.md --dry-run --dump-plan plan.json  # Analyze
cat plan.json                                      # Review
aca run plan.json                                  # Execute

# Interactive mode
aca interactive

# Checkpoint management
aca checkpoint list                    # List available checkpoints
aca checkpoint create "description"    # Create manual checkpoint
aca checkpoint resume <checkpoint-id>  # Resume from specific checkpoint
aca checkpoint resume --latest         # Resume from latest checkpoint
```

#### Library Interface

```rust
use automatic_coding_agent::{AgentSystem, AgentConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the agent system
    let config = AgentConfig::default();
    let agent = AgentSystem::new(config).await?;

    // Create and process a task
    let task_id = agent.create_and_process_task(
        "Implement feature",
        "Add new functionality to the codebase"
    ).await?;

    println!("Task completed: {}", task_id);
    Ok(())
}
```

## Intelligent Task Parsing

Analyze complex task files using LLM-powered semantic understanding:

```bash
# Auto-detect and parse .claude/tasks.md files
aca run .claude/tasks.md

# Add context for better analysis
aca run tasks.md --use-intelligent-parser \
    --context "React + Node.js stack" \
    --context "team of 5, 6 month timeline"

# Generate and examine execution plan
aca run tasks.md --dry-run --dump-plan plan.json
cat plan.json

# Execute the approved plan
aca run plan.json
```

### Benefits

- 🧠 **Semantic Understanding**: Analyzes task meaning, not just text patterns
- 🔗 **Dependency Detection**: Automatically identifies task relationships
- 📊 **Smart Prioritization**: Context-aware priority and complexity assessment
- ⚡ **Optimal Execution**: Determines best execution strategy (sequential/parallel)
- 📋 **Plan Review**: Dump, review, modify plans before execution
- 🎯 **Hierarchical Tasks**: Detects parent-child relationships

**See**: [Intelligent Task Parsing Guide](docs/user-guide/intelligent-task-parsing.md)

## Architecture Overview

The system consists of several key components:

### 1. Task Management (`src/task/`)
- **TaskTree**: Hierarchical task organization with dynamic subtask creation
- **TaskManager**: Central orchestration with event-driven architecture
- **TaskScheduler**: Intelligent prioritization with 6-factor weighted scoring
- **TaskExecution**: Resource allocation and LLM provider integration

### 2. Session Persistence (`src/session/`)
- **SessionManager**: Complete session lifecycle management
- **PersistenceManager**: Atomic file operations with transaction support
- **RecoveryManager**: State validation and corruption recovery
- **SessionMetadata**: Version tracking and performance metrics

### 3. LLM Provider Abstraction (`src/llm/`)
- **LLMProvider**: Unified interface across Claude Code, OpenAI Codex, and local models
- **ProviderConfig**: Provider-specific configuration and capabilities
- **Rate Limiting**: Built-in rate limiting and cost optimization
- **Error Recovery**: Automatic retry and fallback mechanisms

### 4. Claude Code Integration (`src/claude/`)
- **ClaudeCodeInterface**: Direct integration with Claude Code headless mode
- **RateLimiter**: Sophisticated rate limiting with adaptive backoff
- **UsageTracker**: Comprehensive usage monitoring and cost tracking
- **ContextManager**: Intelligent context window management

### 5. CLI Interface (`src/cli/`)
- **Command Processing**: Full CLI argument parsing and validation
- **Intelligent Parser**: LLM-based task decomposition and analysis
- **Execution Plans**: Dump, review, and load execution plans (JSON/TOML)
- **Configuration Management**: TOML-based configuration with defaults
- **Interactive Mode**: User-friendly interface for task management
- **Progress Reporting**: Real-time task progress and system status

### 6. High-Level Integration (`src/integration.rs`)
- **AgentSystem**: Main orchestration layer combining all subsystems
- **SystemStatus**: Comprehensive system monitoring and health checks
- **Task Processing**: End-to-end task lifecycle management

## Project Status

### ✅ Completed Deliverables
- **1.1 Architecture Overview**: Complete system design and specifications
- **1.2 Task Management System**: Dynamic task trees with intelligent scheduling
- **1.3 Session Persistence System**: Atomic persistence with recovery capabilities
- **1.4 Claude Code Integration**: Comprehensive Claude Code interface with rate limiting
- **LLM Provider Abstraction**: Multi-provider support with unified API
- **CLI Interface**: Full command-line interface with configuration management

### 🚧 In Development
- **Docker Deployment System**: Containerized execution environment
- **Advanced Task Processing**: Enhanced task execution with real-world testing
- **Web Interface**: Optional web-based task monitoring and control

### 📋 Current Implementation Status
- **Core Systems**: All major components implemented and tested
- **Integration Tests**: Comprehensive test suite covering all modules
- **Documentation**: Complete API documentation and design specs
- **CLI Functionality**: Fully functional command-line interface
- **Configuration**: TOML-based configuration with sensible defaults

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Install development dependencies
cargo install cargo-audit

# Run full test suite
cargo test

# Check code quality
cargo clippy --all-targets --all-features
cargo fmt --all --check

# Generate documentation
cargo doc --no-deps --all-features --open
```

### Running Tests

The project uses test categories to organize tests based on their dependencies:

```bash
# Run all tests (excluding Claude integration tests)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests (non-Claude)
cargo test --test cli_functionality --test config_generation_integration --test config_toml_integration

# Run only Claude integration tests (requires Claude CLI)
cargo test --test setup_commands_integration --test backup_strategy_integration --test error_handling_integration --test claude_integration

# Run specific test file
cargo test --test config_toml_integration

# Check code without building
cargo check
```

**Test Categories**:
- **Unit Tests**: Standard tests with no external dependencies
- **Integration Tests**: System integration tests (non-Claude dependencies)
- **Claude Integration Tests**: Tests requiring Claude CLI installation (tagged with `#[tag(claude)]`)

See [Testing Guide](docs/testing.md) for detailed information about test setup, categories, and CI configuration.

## Documentation

- **[Architecture Design](docs/design/)**: Detailed system design documents
- **[Session Logs](docs/sessions/)**: Development progress and implementation notes
- **[Testing Guide](docs/testing.md)**: Test categories, setup, and CI configuration
- **[Usage Guide](docs/usage-guide.md)**: Comprehensive usage examples and best practices
- **[Session Management](docs/session-management.md)**: Session persistence and checkpoint system
- **[API Documentation](https://docs.rs/automatic-coding-agent)**: Generated API docs

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Claude Code](https://claude.ai/code) for AI-assisted development
- Powered by the Rust ecosystem and async/await capabilities
- Inspired by modern task orchestration and persistence patterns
