# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build the project
cargo build

# Build optimized release
cargo build --release

# Run the application
cargo run

# Run tests
cargo test

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

**IMPORTANT**: Always maintain session documentation by creating new dated session files in `docs/sessions/` when starting work on new dates. This ensures continuity and proper tracking of development progress across sessions.

## Current State

This is an early-stage project with basic Rust project structure. The main implementation is planned but not yet built - currently contains only a "Hello, world!" main.rs file. The comprehensive design document in `docs/sessions/0-initial-design.md` outlines the full architecture and implementation plan.
- Ensure clippy passes, and that tests pass, before commiting some work.