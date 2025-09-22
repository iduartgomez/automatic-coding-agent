# Automatic Coding Agent - Usage Guide

A powerful Rust-based tool that automates coding tasks using Claude Code in headless mode. The system provides intelligent task execution with full session persistence and resumability.

## Installation

```bash
# Install from source
git clone <repository-url>
cd automatic-coding-agent
cargo install --path .
```

## Quick Start

### Single Task Execution

Execute any text file as a coding task:

```bash
# Execute a task from any text file
automatic-coding-agent --task-file implement_auth.md --workspace /path/to/project

# Works with any file extension
automatic-coding-agent --task-file bug_report.txt --workspace .
automatic-coding-agent --task-file requirements --workspace .
```

### Multi-Task Execution

Create a task list file with multiple tasks:

```bash
# Execute multiple tasks from a list
automatic-coding-agent --tasks project_todos.md --workspace .
```

## Command Line Options

### Task Input (choose one)
- `--task-file <FILE>` - Execute a single task from any UTF-8 file
- `--tasks <FILE>` - Execute multiple tasks from a task list file
- `--config <FILE>` - Load tasks from TOML configuration file (legacy)

### Execution Options
- `--workspace <DIR>` - Override workspace directory (default: current directory)
- `--interactive` - Run in interactive mode
- `--verbose` - Enable detailed logging output
- `--dry-run` - Show what would be executed without running

### Information Options
- `--help` - Show help message
- `--version` - Show version information
- `--show-config` - Show configuration discovery information

## Usage Examples

### Basic File Operations

**Create a simple file:**
```bash
echo "Create a hello.txt file with 'Hello World'" > task.md
automatic-coding-agent --task-file task.md --workspace .
```

**Generate documentation:**
```bash
echo "Create a comprehensive README.md for a weather app called WeatherWiz" > task.md
automatic-coding-agent --task-file task.md --workspace .
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
automatic-coding-agent --tasks tasks.md --workspace ./my-project
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
automatic-coding-agent --tasks tasks.md --workspace ./my-project
```

The tool will automatically include the entire content of `auth_requirements.md` in the task description, providing Claude with comprehensive context for implementation.

## Task Trees and Subtasks

The automatic-coding-agent has a sophisticated task management system with hierarchical task trees and subtask support. However, the current CLI interface processes tasks sequentially without exposing the advanced task tree functionality.

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

The tool automatically manages sessions with:

- **Auto-save**: Session state saved every 5 minutes
- **Checkpoints**: Progress snapshots every 30 minutes
- **Recovery**: Automatic recovery from previous sessions
- **Persistence**: Complete task history and state tracking

Session files are stored in the workspace directory and can be safely interrupted and resumed.

## Verbose Mode

Use `--verbose` for detailed execution logs:

```bash
automatic-coding-agent --task-file task.md --workspace . --verbose
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
automatic-coding-agent --tasks project_setup.md --workspace . --dry-run
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
- Check workspace write permissions
- Ensure sufficient disk space
- Verify session files aren't corrupted

**Multi-task interruption:**
- Use `--verbose` to see progress
- Check session files for partial completion
- Resume execution - the tool will continue from where it left off

### Getting Help

```bash
# Show detailed help
automatic-coding-agent --help

# Check configuration discovery
automatic-coding-agent --show-config

# Enable verbose logging for debugging
automatic-coding-agent --task-file task.md --verbose
```

## Integration with Development Workflow

### Git Integration

The tool works seamlessly with Git workflows:

```bash
# Create a new branch for automated changes
git checkout -b automated-tasks

# Run your tasks
automatic-coding-agent --tasks feature_implementation.md --workspace .

# Review changes
git diff

# Commit results
git add .
git commit -m "Automated implementation of feature tasks"
```

### CI/CD Integration

Use in automated environments:

```bash
# In CI pipeline
automatic-coding-agent --tasks deployment_tasks.md --workspace . --verbose
```

The tool's session management ensures reliable execution even in containerized environments.