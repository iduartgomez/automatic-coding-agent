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

aca (automatic coding agent) is a Rust-based agentic tool that automates coding tasks using Claude Code in headless mode. The system operates in two distinct modes:

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

aca interfaces with Claude Code in headless mode with rate limiting, adaptive backoff, and usage tracking.

### LLM Provider System

The system includes a flexible LLM provider abstraction layer:

- **Claude Provider Modes**:
  - **CLI Mode (default)**: Uses `claude` CLI command, no API key required
  - **API Mode**: Direct Anthropic API access, requires API key
  - Configure via `CLAUDE_MODE` environment variable or config file

- **Intelligent Task Parser**:
  - LLM-powered task decomposition and analysis
  - Automatic markdown file reference resolution (follows `[text](file.md)` links)
  - Hierarchical task structure creation with dependencies
  - Preserves technical details and success criteria
  - Uses `--append-system-prompt` for clean instruction separation

- **Features**:
  - Automatic dependency mapping (indices → TaskIds)
  - Detail preservation (6 high-level tasks → 42+ detailed subtasks)
  - System message support via `--append-system-prompt`
  - Caching for improved performance

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
examples/             # Usage examples
```

# Instructions

- Ensure clippy passes, and that tests pass, before commiting some work.
- Use conventional commits standard for commit messages.
- Document all functionality properly at module level.
- Keep documentation updated when doing changes.
- Ensure that CI passes after committing.
