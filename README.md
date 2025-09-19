# Automatic Coding Agent

[![CI](https://github.com/automatic-coding-agent/automatic-coding-agent/workflows/CI/badge.svg)](https://github.com/automatic-coding-agent/automatic-coding-agent/actions?query=workflow%3ACI)
[![Security Audit](https://github.com/automatic-coding-agent/automatic-coding-agent/workflows/Security%20Audit/badge.svg)](https://github.com/automatic-coding-agent/automatic-coding-agent/actions?query=workflow%3A%22Security+Audit%22)
[![Crates.io](https://img.shields.io/crates/v/automatic-coding-agent.svg)](https://crates.io/crates/automatic-coding-agent)
[![Documentation](https://docs.rs/automatic-coding-agent/badge.svg)](https://docs.rs/automatic-coding-agent)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust-based agentic tool that automates coding tasks using Claude Code in headless mode. The system operates with dynamic task trees, comprehensive session persistence, and full resumability for long-running automated coding sessions.

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

## Quick Start

### Prerequisites
- Rust 1.75.0 or later
- Cargo

### Installation

```bash
# Clone the repository
git clone https://github.com/automatic-coding-agent/automatic-coding-agent.git
cd automatic-coding-agent

# Build the project
cargo build --release

# Run tests
cargo test

# Run the application
cargo run
```

### Usage

```rust
use automatic_coding_agent::{
    SessionManager, SessionManagerConfig, SessionInitOptions,
    TaskManager, TaskManagerConfig, TaskSpec
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize session manager
    let session_config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "My Coding Session".to_string(),
        workspace_root: std::env::current_dir()?,
        enable_auto_save: true,
        ..Default::default()
    };

    let session_manager = SessionManager::new(
        "/path/to/session".into(),
        session_config,
        init_options
    ).await?;

    // Create and execute tasks
    let task_spec = TaskSpec {
        title: "Implement feature X".to_string(),
        description: "Add new functionality...".to_string(),
        // ... task configuration
    };

    let task_id = session_manager
        .task_manager()
        .create_task(task_spec, None)
        .await?;

    // Session state is automatically persisted
    Ok(())
}
```

## Architecture Overview

The system consists of several key components:

### 1. Task Management (`src/task/`)
- **TaskTree**: Hierarchical task organization with dynamic subtask creation
- **TaskManager**: Central orchestration with event-driven architecture
- **TaskScheduler**: Intelligent prioritization with 6-factor weighted scoring
- **TaskExecution**: Resource allocation and Claude Code interface integration

### 2. Session Persistence (`src/session/`)
- **SessionManager**: Complete session lifecycle management
- **PersistenceManager**: Atomic file operations with transaction support
- **RecoveryManager**: State validation and corruption recovery
- **SessionMetadata**: Version tracking and performance metrics

### 3. Core Types (`src/task/types.rs`)
- Rich type system for tasks, priorities, and execution states
- Comprehensive error handling with structured error types
- Serializable data structures for persistence

## Project Status

### ✅ Completed Deliverables
- **1.1 Architecture Overview**: Complete system design and specifications
- **1.2 Task Management System**: Dynamic task trees with intelligent scheduling
- **1.3 Session Persistence System**: Atomic persistence with recovery capabilities

### 🚧 In Development
- **1.4 Claude Code Integration**: Headless SDK integration with rate limiting
- **1.5 Docker Deployment System**: Containerized execution environment
- **1.6 CLI Frontend**: User interface and session control

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

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Performance benchmarks
cargo test --release -- --ignored benchmark
```

## Documentation

- **[Architecture Design](docs/design/)**: Detailed system design documents
- **[Session Logs](docs/sessions/)**: Development progress and implementation notes
- **[API Documentation](https://docs.rs/automatic-coding-agent)**: Generated API docs

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Claude Code](https://claude.ai/code) for AI-assisted development
- Powered by the Rust ecosystem and async/await capabilities
- Inspired by modern task orchestration and persistence patterns