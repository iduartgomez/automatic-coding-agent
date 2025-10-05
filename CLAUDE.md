# aca - Automatic Coding Agent

Rust-based agentic tool automating coding tasks via multiple LLM providers (Claude, OpenAI, Ollama).

## Quick Start

```bash
# Development
cargo build                    # Build project
cargo test --lib              # Run unit tests
cargo clippy                  # Lint code
cargo fmt                     # Format code

# Usage
aca run <file>                      # Execute file (auto-detects: .md/.txt=tasks, .json/.toml=plan)
aca run tasks.md --verbose          # Show real-time subprocess output
aca run tasks.md -w /path/to/dir    # Specify workspace directory
aca interactive                     # Run in interactive mode
aca checkpoint list                 # List checkpoints
aca checkpoint create "desc"        # Create checkpoint
aca checkpoint resume <id>          # Resume from specific checkpoint
aca checkpoint resume <id> --latest # Resume from latest

# Verbose Mode (Real-time Output)
aca run tasks.md --verbose          # See Claude Code's JSONL stream in real-time
# Shows tool uses, responses, and progress as they happen
# All output still captured to .aca/sessions/{id}/logs/ for audit

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
- **Claude CLI (default)**: Uses `claude` command, no API key needed
- **Claude API**: Set `CLAUDE_MODE=API` + `ANTHROPIC_API_KEY` for direct API access
- **OpenAI**: Set `OPENAI_API_KEY` for GPT-4 and other models
- **Ollama**: Local model execution for privacy
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

### 📊 Comprehensive Audit Trail
- **Subprocess output**: Real-time streaming with `--verbose` flag
- **Full command logging**: Reproducible bash scripts for each task
- **Complete stdout/stderr**: No truncation, saved to separate files
- **Tool use tracking**: Captures all Claude Code operations (Write, Edit, Bash, etc.)
- **Performance metrics**: Token usage, costs, execution time per task

## File Structure

When running tasks, `aca` creates the following structure in your workspace:

```
workspace/
├── .aca/                                    # All session data
│   └── sessions/
│       └── {session-id}/
│           ├── meta/
│           │   └── session.json                           # Session state
│           ├── checkpoints/
│           │   └── {checkpoint-id}.json                   # Checkpoint snapshots
│           ├── temp/                                      # Temporary files
│           └── logs/
│               └── claude_interactions/
│                   ├── claude-subprocess-{task-id}.log     # Human-readable summary
│                   ├── claude-subprocess-{task-id}.stdout.json  # Full JSON/JSONL output
│                   ├── claude-subprocess-{task-id}.stderr.txt   # Error output
│                   ├── claude-subprocess-{task-id}.command.sh   # Reproducible command
│                   └── claude-subprocess-{task-id}.tools.json   # Tool uses (when enabled)
```

**Important Notes:**
- All files are written to `.aca/` directory (gitignored by default)
- **Audit files per task**: 5 files provide complete execution history
- **Real-time output**: Use `--verbose` flag to see subprocess output as it happens
- **Tool tracking**: Enabled by default, captures all Write/Edit/Bash/Read operations
- Console output (via `RUST_LOG`) goes to stdout/stderr, not files
- Session files are only created when tasks are actually executed

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

## Docker Integration (Planned)

Docker containerization is a planned feature for isolated execution:
- `/repos` (RO): Source repositories
- `/workspace` (RW): Working directory
- `/session` (RW): Persistent state (maps to `.aca/`)

Current implementation uses direct CLI/API execution without containers.

## Troubleshooting

**"ANTHROPIC_API_KEY required"** → Using API mode but no key set. Either:
- Set `CLAUDE_MODE=CLI` (default), or
- Provide `ANTHROPIC_API_KEY=sk-xxx`

**"No such file or directory: claude"** → Install Claude Code CLI from claude.ai/code

**JSON parsing errors** → Parser handles both direct JSON and escaped JSON from CLI

**Dependency errors** → Dependencies mapped via UUID v5, ensure task titles are unique
