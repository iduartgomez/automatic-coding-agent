# ACA (Automatic Coding Agent) - Usage Guide

A powerful Rust-based tool that automates coding tasks using multiple LLM providers (Claude Code CLI/API, OpenAI Codex, Ollama). The system provides intelligent task execution with full session persistence, resumability, and unified execution plans.

## Installation

```bash
# Install from source
git clone <repository-url>
cd aca
cargo install --path .

# The binary is named 'aca'
aca --help
```

## Quick Start

### Single Task Execution

Execute any text file as a coding task:

```bash
# Execute a task from any text file
aca run examples/task-inputs/single_task.md -w /path/to/project

# Works with any file extension
aca run bug_report.txt -w .
aca run requirements -w .
```

### Intelligent Task Parsing

Analyze complex task files with LLM-powered understanding:

```bash
# Auto-detect intelligent parsing for task lists
aca run .claude/tasks.md

# Explicit intelligent parsing with context
aca run project-tasks.md --use-intelligent-parser \
    --context "full-stack web application" \
    --context "React + Node.js stack"

# Analyze, review, and execute workflow
aca run tasks.md --dry-run --dump-plan plan.json  # Step 1: Analyze
cat plan.json                                      # Step 2: Review
aca run plan.json                                  # Step 3: Execute
```

### Selecting Providers

```bash
# Use Claude Code CLI (default)
aca --provider claude-code run tasks.md --use-intelligent-parser

# Use OpenAI Codex CLI with explicit model override
aca --provider openai-codex --model gpt-5 run main-tasks.md --use-intelligent-parser
```

### Multi-Task Execution

Create a task list file with multiple tasks:

```bash
# Execute multiple tasks from a list
aca run examples/task-inputs/task_list.md -w .
```

### Container Execution

Run tasks inside isolated Docker/Podman containers:

```bash
# Execute in default container
aca run tasks.md --use-containers

# Use custom container image
aca run tasks.md --use-containers --container-image ubuntu:22.04
```

### Structured Configuration

Use configuration files with setup commands:

```bash
# Execute using structured TOML configuration
aca run file.md -c examples/configurations/default-config.toml
```

## Command Line Options

### Commands

- `aca run <FILE>` - Execute a file (auto-detects task, task list, or execution plan)
- `aca interactive` - Run in interactive mode
- `aca checkpoint list` - List available checkpoints
- `aca checkpoint create <DESC>` - Create manual checkpoint
- `aca checkpoint resume <ID>` - Resume from specific checkpoint
- `aca checkpoint resume --latest` - Resume from latest checkpoint
- `aca show-config` - Show configuration discovery information

### Run Options

- `-w, --workspace <DIR>` - Override workspace directory (default: current directory)
- `-c, --config <FILE>` - Configuration file path
- `-v, --verbose` - Enable detailed logging output
- `-n, --dry-run` - Show what would be executed without running
- `--use-intelligent-parser` - Use LLM-based task parser
- `--force-naive-parser` - Force naive parser even for complex files
- `--context <HINT>` - Context hints for intelligent parser (repeatable)
- `--dump-plan <FILE>` - Dump execution plan to file (JSON or TOML)
- `--use-containers` - Execute tasks inside a container (Docker/Podman)
- `--container-image <IMAGE>` - Container image to use (default: alpine:latest)

### Global Options

- `--provider <PROVIDER>` - Override LLM provider (claude-code, openai-codex, anthropic, local-model)
- `--model <MODEL>` - Override model for selected provider
- `-h, --help` - Show help message
- `-V, --version` - Show version information

## Example Files

ACA includes comprehensive examples in the `examples/` directory:

```
examples/
├── task-inputs/           # Task input format examples
│   ├── single_task.md    # Complete feature specification
│   └── task_list.md      # Multiple tasks with references
├── configurations/       # Configuration examples
│   ├── default-config.toml # Standard configuration template
│   └── simple-tasks.toml   # Legacy format (outdated)
└── references/           # Reference files for tasks
    ├── memory_leak_analysis.md
    └── caching_strategy.txt
```

**Try the examples:**

```bash
# Single comprehensive task
aca run examples/task-inputs/single_task.md --dry-run

# Multiple tasks with references
aca run examples/task-inputs/task_list.md --dry-run

# Structured configuration
aca run task.md -c examples/configurations/default-config.toml --dry-run
```

See the [examples README](../examples/README.md) for detailed documentation.

## ExecutionPlan Architecture

ACA uses a unified ExecutionPlan system that processes all input types consistently:

**Task Inputs** → **ExecutionPlan** → **AgentSystem.execute_plan()**

This means:
- Single files, task lists, and configurations all use the same execution engine
- Consistent behavior and error handling across all modes
- Rich metadata and execution mode support
- Better extensibility for future features

## Usage Examples

### Basic File Operations

**Create a simple file:**

```bash
echo "Create a hello.txt file with 'Hello World'" > task.md
aca run task.md -w .
```

**Generate documentation:**

```bash
echo "Create a comprehensive README.md for a weather app called WeatherWiz" > task.md
aca run task.md -w .
```

### Multi-Task Projects

Create a `tasks.md` file with multiple tasks:

```markdown
# Project Setup Tasks

- [ ] Create a Python script called `hello.py` that prints "Hello World"
- [ ] Create a JSON config file called `config.json` with API settings
- [ ] Write a shell script called `run.sh` that executes the Python script
- [ ] Create a `.gitignore` file with common Python patterns
- [ ] Generate a `requirements.txt` file with common dependencies
```

Execute all tasks:

```bash
aca run tasks.md -w ./my-project
```

### Supported Task List Formats

The tool supports various task list formats:

**Markdown format:**

```markdown
- [ ] Incomplete task
- [x] Completed task
- [ ] Another task

* Bullet point task
```

**Org-mode format:**

```org
* TODO First task
* DONE Completed task
* Another task
```

**Numbered lists:**

```
1. First task
2. Second task
3. Third task
```

**Plain text:**

```
Task one
Task two
Task three
```

### Task References

You can reference external files in two ways:

#### 1. Arrow Syntax (`->`)
Reference files for detailed task context using the `-> reference_file` syntax:

**Basic reference syntax:**

```markdown
- [ ] Implement user authentication -> auth_requirements.md
- [ ] Fix memory leak issue -> bug_analysis.txt
- [ ] Add API documentation -> api_spec.json
```

**Example with detailed requirements:**

Create `auth_requirements.md`:

```markdown
# Authentication Requirements

## Features Needed

- JWT token-based authentication
- Password hashing with bcrypt
- Session management
- Role-based access control (RBAC)

## Technical Specifications

- Use FastAPI for endpoints
- PostgreSQL for user storage
- Redis for session caching
- Add rate limiting for login attempts

## API Endpoints Required

- POST /auth/login
- POST /auth/register
- POST /auth/logout
- GET /auth/profile
- PUT /auth/profile
```

Create `tasks.md`:

```markdown
# Development Tasks

- [ ] Implement user authentication system -> auth_requirements.md
- [ ] Create database models -> db_schema.md
- [ ] Add frontend components -> ui_mockups.md
```

**How it works:**

- The reference file content is automatically included in the task description
- Supports any UTF-8 text file format
- Relative paths are resolved relative to the task list file
- Absolute paths are supported
- Reference files can contain detailed specifications, code examples, or requirements

### Task References in Action

**Real example with authentication system:**

1. Create detailed requirements file (`auth_requirements.md`)
2. Reference it in your task list:

```markdown
# Development Tasks

- [ ] Implement user authentication system -> auth_requirements.md
- [ ] Create database schema -> database_schema.md
- [ ] Add unit tests for auth functions
```

3. Execute with full context:

```bash
aca run tasks.md -w ./my-project
```

The tool will automatically include the entire content of `auth_requirements.md` in the task description, providing Claude with comprehensive context for implementation.

#### 2. Markdown File Links

The intelligent parser automatically resolves markdown-style file references in task descriptions:

**Link syntax:**
```markdown
# Development Tasks

- [ ] Review the [API specification](docs/api-spec.md) and implement endpoints
- [ ] Follow [coding standards](CONTRIBUTING.md) for all changes
- [ ] See [architecture diagram](docs/architecture.png) for system overview
```

**How it works:**
- Markdown links like `[text](file.md)` are automatically detected
- File contents are loaded and included in task context
- Works with relative and absolute paths
- Supports any text-based file format
- Images and binary files are referenced by path only

**Example with intelligent parser:**
```bash
# Task file with markdown links
cat > tasks.md << 'EOF'
# Implementation Tasks

- [ ] Implement authentication following [security guidelines](docs/security.md)
- [ ] Add API endpoints per [API spec](docs/api.json)
- [ ] Update [README](README.md) with new features
EOF

# Parse with intelligent analysis
aca run tasks.md --use-intelligent-parser \
    --context "security-focused implementation"
```

The intelligent parser will:
1. Detect all markdown file links
2. Load file contents
3. Include them in task analysis
4. Provide rich context to the LLM

## Task Trees and Subtasks

The aca has a sophisticated task management system with hierarchical task trees and subtask support. However, the current CLI interface processes tasks sequentially without exposing the advanced task tree functionality.

### Current Functionality

**Sequential Task Processing:**

- Tasks from task lists are processed one by one
- Each task is independent and complete
- Session management tracks all task progress
- Failed tasks don't block subsequent tasks

### Advanced Task Management (API Level)

The underlying system supports:

- **Parent-child task relationships**
- **Dynamic subtask creation**
- **Dependency resolution**
- **Context inheritance**
- **Automatic completion when all subtasks finish**

These features are available through the programmatic API but not yet exposed through the task list file format.

### Future Enhancement Possibilities

**Potential task hierarchy syntax:**

```markdown
# Main Project

- [ ] Setup web application
  - [ ] Create backend API
    - [ ] User authentication
    - [ ] Database models
    - [ ] API endpoints
  - [ ] Build frontend
    - [ ] React components
    - [ ] State management
    - [ ] Styling
- [ ] Add testing
- [ ] Deploy to production
```

**Note:** This hierarchical syntax is not currently supported but represents the direction for future enhancements.

## Structured Configuration Mode

Use TOML configuration files for comprehensive system setup:

### Basic Configuration

```toml
workspace_path = "/path/to/your/project"

# Setup commands run before task processing
setup_commands = [
    { name = "install_deps", command = "npm install" },
    { name = "build_project", command = "cargo build" }
]

[session_config]
auto_save_interval_minutes = 5
auto_checkpoint_interval_minutes = 30
enable_crash_recovery = true

[task_config]
auto_retry_failed_tasks = true
max_concurrent_tasks = 3

[claude_config.rate_limits]
max_tokens_per_minute = 40000
max_requests_per_minute = 50
```

### Configuration Benefits

- **Setup automation**: Run environment preparation commands
- **System configuration**: Fine-tune session, task, and Claude settings
- **Reproducible environments**: Share configurations across teams
- **Production ready**: Optimized settings for different environments

### Usage

```bash
# Execute with structured configuration
aca run task.md -c project-config.toml

# Test configuration without execution
aca run task.md -c project-config.toml --dry-run --verbose
```

See [`examples/configurations/`](../examples/configurations/) for complete examples.

### Advanced Examples

**Complex web application:**

```markdown
# Web App Development Tasks

- [ ] Create HTML structure in `index.html` with navigation and main content
- [ ] Add CSS styling in `styles.css` with responsive design
- [ ] Implement JavaScript functionality in `app.js` for user interactions
- [ ] Create API endpoints in `server.py` using Flask
- [ ] Add database models in `models.py` with SQLAlchemy
- [ ] Write unit tests in `test_app.py` covering main functionality
- [ ] Create deployment configuration in `Dockerfile`
```

**Data processing pipeline:**

```markdown
# Data Pipeline Tasks

- [ ] Create data ingestion script `ingest.py` to read CSV files
- [ ] Implement data cleaning functions in `clean.py`
- [ ] Add data transformation logic in `transform.py`
- [ ] Create visualization dashboard in `dashboard.py` using Plotly
- [ ] Generate summary reports in `reports.py`
- [ ] Add configuration file `pipeline.config.json`
```

## Session Management

ACA provides comprehensive session management with automatic recovery:

### Automatic Management

- **Auto-save**: Session state saved every 5 minutes
- **Auto-checkpoints**: Progress snapshots every 30 minutes
- **Crash recovery**: Automatic recovery from unexpected shutdowns
- **Complete persistence**: Task history, progress, and file system state

### Manual Checkpoint Control

```bash
# List available checkpoints
aca checkpoint list

# Create a manual checkpoint
aca checkpoint create "Feature implementation complete"

# Resume from latest checkpoint
aca checkpoint resume --latest

# Resume from specific checkpoint
aca checkpoint resume checkpoint-abc-123
```

### Session Storage

Session data is stored in the `.aca/` directory within your workspace:

```
your-project/
├── .aca/
│   ├── sessions/     # Session data
│   └── checkpoints/  # Checkpoint storage
└── your-files/
```

Sessions can be safely interrupted and resumed across system restarts.

## Verbose Mode

Use `--verbose` for detailed execution logs:

```bash
aca run task.md -w . --verbose
```

This shows:

- Task loading and parsing details
- Claude Code integration status
- File operations and progress
- Session management activities
- Error details and recovery attempts

## Dry Run Mode

Test your tasks without execution:

```bash
aca run project_setup.md -w . --dry-run
```

This will:

- Parse and validate all tasks
- Show what would be executed
- Verify file access and permissions
- Display estimated execution plan

## Best Practices

### Task Description Guidelines

**Be specific and actionable:**

```markdown
✅ Good: "Create a Python FastAPI server with user authentication endpoints"
❌ Bad: "Make an API"
```

**Include context and requirements:**

```markdown
✅ Good: "Create a responsive CSS layout with header, sidebar, and main content areas"
❌ Bad: "Add some CSS"
```

**Specify file names and locations:**

```markdown
✅ Good: "Create database models in `models/user.py` using SQLAlchemy"
❌ Bad: "Add database stuff"
```

### Multi-Task Organization

**Group related tasks:**

```markdown
# Frontend Tasks

- [ ] Create HTML structure
- [ ] Add CSS styling
- [ ] Implement JavaScript

# Backend Tasks

- [ ] Set up database
- [ ] Create API endpoints
- [ ] Add authentication
```

**Use logical ordering:**

```markdown
- [ ] Create project structure
- [ ] Add configuration files
- [ ] Implement core functionality
- [ ] Add tests
- [ ] Create documentation
```

## Troubleshooting

### Common Issues

**Task not executing:**

- Ensure configured LLM provider is accessible:
  - Claude CLI mode: `claude` command is installed
  - Claude API mode: `ANTHROPIC_API_KEY` environment variable is set
  - OpenAI Codex: `CODEX_CLI_PATH` is discoverable in `PATH`
- Check workspace permissions
- Verify task file is UTF-8 encoded
- Use `--verbose` flag to see detailed provider initialization logs

**Session recovery fails:**

- Check workspace write permissions for `.aca/` directory
- Ensure sufficient disk space for session data
- Verify session files aren't corrupted
- Try `aca checkpoint list` to see available recovery points

**Multi-task interruption:**

- Use `--verbose` to see detailed progress
- Check `.aca/sessions/` for partial completion status
- Resume with `aca checkpoint resume --latest` for latest checkpoint
- Use `aca checkpoint resume <checkpoint-id>` for specific recovery point

**Configuration issues:**

- Validate TOML syntax in configuration files
- Check setup command permissions and dependencies
- Use `--dry-run` to test configuration without execution
- Verify workspace paths and file permissions

### Getting Help

```bash
# Show detailed help
aca --help
aca run --help

# Check configuration discovery
aca show-config

# List available checkpoints
aca checkpoint list

# Enable verbose logging for debugging
aca run task.md --verbose

# Test commands without execution
aca run my_tasks.md --dry-run --verbose
```

## Integration with Development Workflow

### Git Integration

ACA works seamlessly with Git workflows:

```bash
# Create a new branch for automated changes
git checkout -b automated-tasks

# Run your tasks with checkpointing
aca run feature_implementation.md -w .

# Create checkpoint before review
aca checkpoint create "Implementation complete, ready for review"

# Review changes
git diff

# Commit results
git add .
git commit -m "Automated implementation of feature tasks"
```

### Iterative Development

```bash
# Phase 1: Initial implementation
aca run phase1_tasks.md
aca checkpoint create "Phase 1 complete"

# Phase 2: Add features
aca run phase2_tasks.md
aca checkpoint create "Phase 2 complete"

# Phase 3: Testing and refinement
aca run phase3_tasks.md

# View progress
aca checkpoint list
```

### Team Collaboration

```bash
# Share configuration for consistent environments
aca run task.md -c team-config.toml

# Resume colleague's work from checkpoint
aca checkpoint resume checkpoint-shared-123

# Create structured task assignments
aca run team-assignments.md --verbose
```

### CI/CD Integration

Use in automated environments:

```bash
# In CI pipeline
aca run deployment_tasks.md -w . --verbose
```

The tool's session management ensures reliable execution even in containerized environments.

## Workspace Structure

ACA organizes its data in a predictable structure within your project:

```
your-project/
├── .aca/                     # ACA data directory
│   ├── sessions/            # Session persistence data
│   │   └── session_xyz/     # Individual session data
│   └── checkpoints/         # Checkpoint storage
├── examples/                # Built-in examples (if present)
│   ├── task-inputs/        # Task input examples
│   ├── configurations/     # Configuration examples
│   └── references/         # Reference files
└── your-project-files/     # Your actual project files
```

**Key directories:**
- `.aca/` - All ACA data (can be safely added to .gitignore)
- `examples/` - Built-in examples and templates (optional)
- Session data includes complete task history and execution state
- Checkpoints can be used for recovery and collaboration

## Intelligent Task Parsing

### Overview

The intelligent parser uses LLM (Claude) to semantically understand task structures:

- **Hierarchical Detection**: Automatically identifies parent-child relationships
- **Dependency Analysis**: Detects which tasks depend on others
- **Priority Assignment**: Context-aware prioritization
- **Complexity Estimation**: Estimates difficulty per task
- **Execution Strategy**: Determines optimal Sequential/Parallel/Intelligent mode

### Basic Usage

```bash
# Auto-enable for task lists
aca run project.md

# Explicit intelligent parsing
aca run tasks.md --use-intelligent-parser

# Add context hints for better analysis
aca run tasks.md --use-intelligent-parser \
    --context "microservices architecture" \
    --context "Python + FastAPI backend" \
    --context "team of 3 developers"
```

### Execution Plan Workflow

**Step 1: Analyze and dump plan**
```bash
aca run .claude/tasks.md \
    --use-intelligent-parser \
    --dry-run \
    --dump-plan execution-plan.json \
    --verbose
```

**Step 2: Review the plan**
```bash
# View full plan
cat execution-plan.json

# Extract specific info
jq '.task_specs[] | {title, priority: .metadata.priority}' execution-plan.json
```

**Step 3: Modify if needed (optional)**
```bash
# Edit priorities, execution mode, etc.
vim execution-plan.json
```

**Step 4: Execute the approved plan**
```bash
aca run execution-plan.json --verbose
```

### Plan Formats

Plans can be dumped and loaded in two formats:

```bash
# JSON format (recommended for readability)
aca run tasks.md --dry-run --dump-plan plan.json
aca run plan.json

# TOML format
aca run tasks.md --dry-run --dump-plan plan.toml
aca run plan.toml
```

### When to Use Intelligent Parser

**Use intelligent parser when:**
- ✅ Complex multi-phase projects
- ✅ Tasks with implicit dependencies
- ✅ Need to review execution plan before running
- ✅ Want priority/complexity estimates
- ✅ Sharing plans with team for review

**Use naive parser when:**
- ✅ Single simple task
- ✅ LLM API unavailable
- ✅ Want faster processing
- ✅ Explicit TOML configuration

### Examples

**Example 1: Analyze EU Products App Tasks**
```bash
export ANTHROPIC_API_KEY=your_key

aca run /path/to/eu-products/app/.claude/tasks.md \
    --use-intelligent-parser \
    --context "full-stack Flutter + Rust app" \
    --context "data-intensive, 6 months" \
    --dry-run \
    --dump-plan eu-products-plan.json

# Review
cat eu-products-plan.json

# Execute
aca run eu-products-plan.json
```

**Example 2: Team Review Workflow**
```bash
# Developer: Analyze and commit plan
aca run sprint-tasks.md --dry-run --dump-plan sprint-plan.json
git add sprint-plan.json
git commit -m "Add sprint execution plan for review"
git push

# Team: Review in PR, approve

# Execute approved plan
git pull
aca run sprint-plan.json
```

**See full documentation**: [Intelligent Task Parsing Guide](user-guide/intelligent-task-parsing.md)
