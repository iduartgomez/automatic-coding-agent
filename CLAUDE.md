This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build the project
cargo build

# Build optimized release
cargo build --release

# Run the application
cargo run

# Run all tests (unit + integration)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --tests

# Run specific integration test suite
cargo test --test config_toml_integration

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy
```

## Project Architecture

This is a Rust-based agentic tool that automates coding tasks using Claude Code in headless mode. The system operates in two distinct modes:

1. **Host-side session initializer** - Manages Docker environments and session persistence
2. **In-container agent** - Executes tasks using a dynamic task tree with full persistence and resumability

### Core Components

- **CLI Frontend & Session Manager (Host)**: Handles Docker lifecycle, volume management, and session persistence
- **Agent Runtime (Container)**: Executes task automation logic, interfaces with Claude Code headless mode, manages dynamic task trees
- **Task Management System**: Dynamic task tree with subtask creation, dependency resolution, and progress tracking
- **Session Persistence**: Complete state management including task hierarchy, Claude Code context, file system state, and execution logs

### Key Architecture Details

The system uses Docker containers with volume mounts:

- `/repos` (RO) - Source repositories
- `/workspace` (RW) - Working directory
- `/session` (RW) - Persistent session data
- `/logs` (RW) - Session logs and outputs

Tasks are managed in a hierarchical tree structure with support for:

- Dynamic subtask creation
- Dependency resolution
- Context inheritance
- Real-time progress tracking
- Full persistence and resumability

The agent interfaces with Claude Code in headless mode with rate limiting, adaptive backoff, and usage tracking.

## Documentation Structure

- **Core design documents**: Located in `docs/design/` directory
- **Session documentation**: Located in `docs/sessions/` directory

**CRITICAL SESSION DOCUMENTATION REQUIREMENT**:
- **ALWAYS** create or update session documentation in `docs/sessions/` directory
- **MUST** create a new session file `docs/sessions/YYYY-MM-DD-session-topic.md` when starting work on any new date
- **MUST** document objectives, progress, implementation details, and outcomes for each session
- **MUST** update the session log throughout the work session, not just at the end
- This ensures continuity and proper tracking of development progress across sessions

Example: `docs/sessions/2025-09-19-llm-abstraction-implementation.md`

## Current State

This project has a fully implemented Rust architecture with the following components:

### ✅ **Implemented Systems**
- **Task Management System**: Complete hierarchical task tree with scheduling, dependencies, and error handling
- **Session Persistence**: Full state management with checkpoints and recovery
- **Claude Integration**: LLM interface with rate limiting and context management
- **Agent Configuration**: TOML-based configuration system with serialization/deserialization
- **Setup Commands**: Pre-execution command system with retry, skip, and backup strategies

### 🧪 **Testing Infrastructure**
- **85 Total Tests**: 51 unit tests + 34 integration tests
- **Integration Test Suites**:
  - `config_toml_integration.rs` - TOML configuration testing (5 tests)
  - `config_generation_integration.rs` - Config generation testing (5 tests)
  - `backup_strategy_integration.rs` - Backup strategy testing (7 tests)
  - `error_handling_integration.rs` - Error handling testing (8 tests)
  - `setup_commands_integration.rs` - Setup commands testing (9 tests)

### 📁 **Project Structure**
```
src/
├── integration.rs      # Main agent system integration
├── task/              # Task management system
├── session/           # Session persistence system
├── claude/            # Claude Code integration
├── llm/              # LLM abstraction layer
└── lib.rs            # Library exports

tests/                 # Integration tests
├── backup_strategy_integration.rs
├── config_generation_integration.rs
├── config_toml_integration.rs
├── error_handling_integration.rs
└── setup_commands_integration.rs

examples/             # Usage examples
├── default-config.toml
└── llm_provider_example.rs
```

### 🔧 **Configuration**
The system supports TOML-based configuration with the `AgentConfig` struct:

```rust
// Load from file
let config = AgentConfig::from_toml_file("config.toml")?;

// Save to file
config.to_toml_file("output.toml")?;
```

# Instructions

- Ensure clippy passes, and that tests pass, before commiting some work.
- Use conventional commits standard for commit messages.
- Document all functionality properly at module level.
- Keep documentation updated when doing changes.