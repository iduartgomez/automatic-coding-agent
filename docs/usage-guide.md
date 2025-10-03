# ACA (Automatic Coding Agent) - Usage Guide

A powerful Rust-based tool that automates coding tasks using Claude Code in headless mode. The system provides intelligent task execution with full session persistence, resumability, and unified execution plans.

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
aca --task-file examples/task-inputs/single_task.md --workspace /path/to/project

# Works with any file extension
aca --task-file bug_report.txt --workspace .
aca --task-file requirements --workspace .
```

### Intelligent Task Parsing

Analyze complex task files with LLM-powered understanding:

```bash
# Auto-detect intelligent parsing for task lists
aca --task-file .claude/tasks.md

# Explicit intelligent parsing with context
aca --task-file project-tasks.md --use-intelligent-parser \
    --context "full-stack web application" \
    --context "React + Node.js stack" \
    --context "6 month timeline"

# Analyze, review, and execute workflow
aca --task-file tasks.md --dry-run --dump-plan plan.json  # Step 1: Analyze
cat plan.json                                              # Step 2: Review
aca --execution-plan plan.json                             # Step 3: Execute
```

### Multi-Task Execution

Create a task list file with multiple tasks:

```bash
# Execute multiple tasks from a list
aca --tasks examples/task-inputs/task_list.md --workspace .
```

### Structured Configuration

Use configuration files with setup commands:

```bash
# Execute using structured TOML configuration
aca --config examples/configurations/default-config.toml
```

## Command Line Options

### Task Input (choose one)

- `--task-file <FILE>` - Execute a single task from any UTF-8 file
- `--tasks <FILE>` - Execute multiple tasks from a task list file
- `--config <FILE>` - Load tasks from TOML configuration file

### Execution Options

- `--workspace <DIR>` - Override workspace directory (default: current directory)
- `--interactive` - Run in interactive mode
- `--verbose` - Enable detailed logging output
- `--dry-run` - Show what would be executed without running

### Session Management

- `--resume <CHECKPOINT_ID>` - Resume from specific checkpoint
- `--continue` - Resume from latest checkpoint
- `--list-checkpoints` - Show available checkpoints
- `--create-checkpoint <DESCRIPTION>` - Create manual checkpoint

### Information Options

- `--help` - Show help message
- `--version` - Show version information
- `--show-config` - Show configuration discovery information

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
aca --task-file examples/task-inputs/single_task.md --dry-run

# Multiple tasks with references
aca --tasks examples/task-inputs/task_list.md --dry-run

# Structured configuration
aca --config examples/configurations/default-config.toml --dry-run
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
aca --task-file task.md --workspace .
```

**Generate documentation:**

```bash
echo "Create a comprehensive README.md for a weather app called WeatherWiz" > task.md
aca --task-file task.md --workspace .
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
aca --tasks tasks.md --workspace ./my-project
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

You can reference external files for detailed task context using the `-> reference_file` syntax:

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
aca --tasks tasks.md --workspace ./my-project
```

The tool will automatically include the entire content of `auth_requirements.md` in the task description, providing Claude with comprehensive context for implementation.

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
# Execute structured configuration
aca --config project-config.toml

# Test configuration without execution
aca --config project-config.toml --dry-run --verbose
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
aca --list-checkpoints

# Create a manual checkpoint
aca --create-checkpoint "Feature implementation complete"

# Resume from latest checkpoint
aca --continue

# Resume from specific checkpoint
aca --resume checkpoint-abc-123
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
aca --task-file task.md --workspace . --verbose
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
aca --tasks project_setup.md --workspace . --dry-run
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

- Ensure Claude CLI is installed and accessible
- Check workspace permissions
- Verify task file is UTF-8 encoded

**Session recovery fails:**

- Check workspace write permissions for `.aca/` directory
- Ensure sufficient disk space for session data
- Verify session files aren't corrupted
- Try `aca --list-checkpoints` to see available recovery points

**Multi-task interruption:**

- Use `--verbose` to see detailed progress
- Check `.aca/sessions/` for partial completion status
- Resume with `aca --continue` for latest checkpoint
- Use `aca --resume <checkpoint-id>` for specific recovery point

**Configuration issues:**

- Validate TOML syntax in configuration files
- Check setup command permissions and dependencies
- Use `--dry-run` to test configuration without execution
- Verify workspace paths and file permissions

### Getting Help

```bash
# Show detailed help
aca --help

# Check configuration discovery
aca --show-config

# List available checkpoints
aca --list-checkpoints

# Enable verbose logging for debugging
aca --task-file task.md --verbose

# Test commands without execution
aca --tasks my_tasks.md --dry-run --verbose
```

## Integration with Development Workflow

### Git Integration

ACA works seamlessly with Git workflows:

```bash
# Create a new branch for automated changes
git checkout -b automated-tasks

# Run your tasks with checkpointing
aca --tasks feature_implementation.md --workspace .

# Create checkpoint before review
aca --create-checkpoint "Implementation complete, ready for review"

# Review changes
git diff

# Commit results
git add .
git commit -m "Automated implementation of feature tasks"
```

### Iterative Development

```bash
# Phase 1: Initial implementation
aca --tasks phase1_tasks.md
aca --create-checkpoint "Phase 1 complete"

# Phase 2: Add features
aca --tasks phase2_tasks.md
aca --create-checkpoint "Phase 2 complete"

# Phase 3: Testing and refinement
aca --tasks phase3_tasks.md

# View progress
aca --list-checkpoints
```

### Team Collaboration

```bash
# Share configuration for consistent environments
aca --config team-config.toml

# Resume colleague's work from checkpoint
aca --resume checkpoint-shared-123

# Create structured task assignments
aca --tasks team-assignments.md --verbose
```

### CI/CD Integration

Use in automated environments:

```bash
# In CI pipeline
aca --tasks deployment_tasks.md --workspace . --verbose
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
aca --tasks project.md

# Explicit intelligent parsing
aca --task-file tasks.md --use-intelligent-parser

# Add context hints for better analysis
aca --tasks tasks.md --use-intelligent-parser \
    --context "microservices architecture" \
    --context "Python + FastAPI backend" \
    --context "team of 3 developers"
```

### Execution Plan Workflow

**Step 1: Analyze and dump plan**
```bash
aca --task-file .claude/tasks.md \
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
aca --execution-plan execution-plan.json --verbose
```

### Plan Formats

Plans can be dumped and loaded in two formats:

```bash
# JSON format (recommended for readability)
aca --tasks tasks.md --dry-run --dump-plan plan.json
aca --execution-plan plan.json

# TOML format
aca --tasks tasks.md --dry-run --dump-plan plan.toml
aca --execution-plan plan.toml
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

aca --task-file /path/to/eu-products/app/.claude/tasks.md \
    --use-intelligent-parser \
    --context "full-stack Flutter + Rust app" \
    --context "data-intensive, 6 months" \
    --dry-run \
    --dump-plan eu-products-plan.json

# Review
cat eu-products-plan.json

# Execute
aca --execution-plan eu-products-plan.json
```

**Example 2: Team Review Workflow**
```bash
# Developer: Analyze and commit plan
aca --tasks sprint-tasks.md --dry-run --dump-plan sprint-plan.json
git add sprint-plan.json
git commit -m "Add sprint execution plan for review"
git push

# Team: Review in PR, approve

# Execute approved plan
git pull
aca --execution-plan sprint-plan.json
```

**See full documentation**: [Intelligent Task Parsing Guide](user-guide/intelligent-task-parsing.md)

