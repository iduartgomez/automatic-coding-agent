# aca - Automatic Coding Agent

Rust-based agentic tool automating coding tasks via Claude Code CLI.

## Quick Start

```bash
# Development
cargo build                    # Build project
cargo test --lib              # Run unit tests
cargo clippy                  # Lint code
cargo fmt                     # Format code

# Testing
cargo test                    # All tests
cargo test --tests            # Integration tests only
cargo test --test <name>      # Specific test suite
```

## Architecture Overview

```
src/
├── cli/               # CLI interface & intelligent task parser
├── llm/              # LLM provider abstraction (Claude, OpenAI)
├── claude/           # Claude Code CLI integration
├── task/             # Task tree management & dependencies
├── session/          # State persistence & recovery
└── integration.rs    # Main agent orchestration
```

## Key Features

### 🤖 Intelligent Task Parser
- **Auto file resolution**: Follows markdown links `[spec](detail.md)` and includes content
- **LLM decomposition**: 6 high-level tasks → 42+ detailed subtasks
- **Dependency mapping**: Automatic TaskId generation and dependency graph
- **Detail preservation**: Keeps technical specs, success criteria, implementation notes

### 🔌 LLM Provider System
- **CLI Mode (default)**: Uses `claude` command, no API key needed
- **API Mode**: Set `CLAUDE_MODE=API` + `ANTHROPIC_API_KEY` for direct API access
- **System prompts**: Uses `--append-system-prompt` for clean instruction separation
- **Caching**: Hash-based response caching for performance

### 📋 Task Management
- Hierarchical task trees with parent-child relationships
- Dynamic subtask creation during execution
- Dependency resolution with cycle detection
- Progress tracking and checkpointing

### 💾 Session Persistence
- Full state serialization (tasks, context, logs)
- Checkpoint and resume from any point
- Crash recovery with automatic state restoration

## Configuration

### Provider Modes
```rust
// Default: CLI mode (no API key)
let config = ProviderConfig::default();

// API mode via environment
CLAUDE_MODE=API ANTHROPIC_API_KEY=sk-xxx cargo run

// API mode via config
let mut config = ProviderConfig::default();
config.additional_config.insert("mode".into(), json!("API"));
```

### Task Files
Tasks support markdown with linked specs:
```markdown
## Database Setup
→ Details: [db-setup.md](db-setup.md)
- PostgreSQL configuration
- Schema creation
```

Parser automatically reads and includes `db-setup.md` content.

## Development Guidelines

### Before Committing
1. ✅ `cargo clippy` passes (no warnings)
2. ✅ `cargo test` passes (all tests)
3. ✅ `cargo fmt` applied
4. ✅ Module-level documentation updated
5. ✅ Use conventional commits (`feat:`, `fix:`, `docs:`, etc.)

### Documentation Requirements
- Update module docstrings when changing behavior
- Keep CLAUDE.md current with architecture changes
- Add examples for new features
- Document breaking changes in commit messages

### Session Logs (Optional)
For major features, create `docs/sessions/YYYY-MM-DD-topic.md` documenting:
- Objectives and approach
- Implementation details
- Outcomes and learnings

## Testing

```bash
# Quick validation
cargo test --lib                                    # Fast unit tests

# Integration tests (require Claude CLI)
cargo test --test intelligent_parser_claude         # Intelligent parser
cargo test --test claude_integration                # Claude interface

# Specific test with output
cargo test test_name -- --nocapture
```

## Docker Integration

The system supports containerized execution with volume mounts:
- `/repos` (RO): Source repositories
- `/workspace` (RW): Working directory
- `/session` (RW): Persistent state
- `/logs` (RW): Execution logs

## Troubleshooting

**"ANTHROPIC_API_KEY required"** → Using API mode but no key set. Either:
- Set `CLAUDE_MODE=CLI` (default), or
- Provide `ANTHROPIC_API_KEY=sk-xxx`

**"No such file or directory: claude"** → Install Claude Code CLI from claude.ai/code

**JSON parsing errors** → Parser handles both direct JSON and escaped JSON from CLI

**Dependency errors** → Dependencies mapped via UUID v5, ensure task titles are unique
