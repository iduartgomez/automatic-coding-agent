# ACA Examples

This directory contains practical examples demonstrating different ways to use the Automatic Coding Agent (ACA). Examples are organized by category to help you quickly find what you need.

## 📁 Directory Structure

```
examples/
├── task-inputs/        # Task input formats (--task-file, --tasks)
├── configurations/     # System configurations (--config)
├── references/         # Reference files used by tasks
└── README.md          # This file
```

## 🚀 Quick Start

### Basic Usage Patterns

```bash
# Single task from file
aca run examples/task-inputs/single_task.md

# Multiple tasks from list
aca run examples/task-inputs/task_list.md

# With configuration file
aca run examples/task-inputs/task_list.md -c examples/configurations/default-config.toml

# Dry run to see what would happen
aca run examples/task-inputs/single_task.md --dry-run

# Verbose output for debugging
aca run examples/task-inputs/task_list.md --verbose

# Run in container
aca run examples/task-inputs/task_list.md --use-containers
```

## 📋 Task Input Examples

### Single Task Files

Single task files contain one complete task description that becomes a single execution unit.

**Example:** [`task-inputs/single_task.md`](task-inputs/single_task.md)
- **Use case**: Complex feature implementation
- **Format**: Any UTF-8 text file (Markdown, plain text, etc.)
- **Command**: `aca run examples/task-inputs/single_task.md`

### Task List Files

Task list files contain multiple tasks that are processed sequentially. Supports various formats:

**Example:** [`task-inputs/task_list.md`](task-inputs/task_list.md)
- **Use case**: Multiple related tasks with references
- **Formats supported**:
  - Markdown: `- [ ] Task description`
  - Org-mode: `* TODO Task description`
  - Numbered: `1. Task description`
  - Plain text: One task per line
  - With references: `Task description -> reference_file.md`
- **Command**: `aca run examples/task-inputs/task_list.md`

#### Task References

Tasks can reference external files for additional context:
```markdown
- [ ] Fix memory leak in data processor -> memory_leak_analysis.md
- [ ] Implement caching strategy -> caching_strategy.txt
```

When ACA processes these tasks, it automatically includes the referenced file content.

## ⚙️ Configuration Examples

### Default Configuration

**Example:** [`configurations/default-config.toml`](configurations/default-config.toml)
- **Use case**: Standard ACA configuration
- **Features**: Session management, task settings, Claude integration
- **Command**: `aca run task.md -c examples/configurations/default-config.toml`


#### Key Configuration Sections

- **`workspace_path`**: Working directory for the agent
- **`setup_commands`**: Commands to run before task processing
- **`session_config`**: Session management and checkpointing
- **`task_config`**: Task execution behavior
- **`claude_config`**: Claude Code integration settings

## 📄 Reference Files

Reference files provide additional context for tasks. They're automatically included when referenced by task lists.

### Analysis Documents

**Example:** [`references/memory_leak_analysis.md`](references/memory_leak_analysis.md)
- **Use case**: Detailed technical analysis
- **Referenced by**: Task list example
- **Contains**: Problem description, investigation findings, solutions

**Example:** [`references/caching_strategy.txt`](references/caching_strategy.txt)
- **Use case**: Implementation specification
- **Referenced by**: Task list example
- **Contains**: Technical requirements, implementation details

## 🎯 Usage Scenarios

### Development Workflow
```bash
# 1. Plan your tasks in a markdown file
vim my_tasks.md

# 2. Run with dry-run to validate
aca run my_tasks.md --dry-run --verbose

# 3. Execute the tasks
aca run my_tasks.md

# 4. Resume if needed
aca checkpoint resume --latest
```

### Configuration-Driven Automation
```bash
# 1. Create configuration with setup commands
vim project-config.toml

# 2. Run with configuration
aca run task.md -c project-config.toml

# 3. Monitor progress
aca checkpoint list
```

### Complex Feature Implementation
```bash
# 1. Write detailed feature specification
vim feature_spec.md

# 2. Execute as single comprehensive task
aca run feature_spec.md --verbose

# 3. Create checkpoint manually if needed
aca checkpoint create "Feature implementation milestone"
```

## 📝 Creating Your Own Examples

### Task Input Best Practices

1. **Single Tasks**: Include comprehensive requirements, technical details, and acceptance criteria
2. **Task Lists**: Break complex work into logical, sequential steps
3. **References**: Create separate files for detailed specifications, analysis, or documentation
4. **Naming**: Use descriptive filenames that indicate the task purpose

### Configuration Best Practices

1. **Workspace**: Set appropriate workspace paths for your environment
2. **Timeouts**: Adjust session and checkpoint intervals based on task complexity
3. **Rate Limits**: Configure Claude API limits based on your usage tier
4. **Setup Commands**: Include environment preparation, dependency installation, etc.

### Example Templates

#### Single Task Template
```markdown
# [Feature/Fix Name]

## Overview
Brief description of what needs to be implemented.

## Requirements
- Functional requirement 1
- Functional requirement 2
- Non-functional requirements

## Technical Details
- Architecture considerations
- Technology choices
- Integration points

## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Testing requirements
```

#### Task List Template
```markdown
# Project: [Project Name]

## Setup Tasks
- [ ] Initialize project structure
- [ ] Install dependencies -> requirements.txt
- [ ] Configure development environment

## Implementation Tasks
- [ ] Implement core feature A
- [ ] Implement feature B -> feature_b_spec.md
- [ ] Add error handling

## Testing Tasks
- [ ] Write unit tests
- [ ] Integration testing
- [ ] Performance testing
```

## 🔧 Advanced Features

### Execution Plan Integration

After the recent ExecutionPlan refactoring, all examples now use the unified execution engine:

- **Task inputs** → ExecutionPlan → AgentSystem.execute_plan()
- **Configurations** → ExecutionPlan → AgentSystem.execute_plan()
- **Consistent behavior** across all input types
- **Rich metadata** and execution mode support

### Session Management

All examples support ACA's session management features:

```bash
# List available checkpoints
aca checkpoint list

# Resume from latest checkpoint
aca checkpoint resume --latest

# Resume from specific checkpoint
aca checkpoint resume checkpoint-id

# Create manual checkpoint
aca checkpoint create "Important milestone"
```

### Workspace Integration

Examples work with ACA's workspace structure:

```
your-workspace/
├── .aca/                    # ACA data directory
│   ├── sessions/           # Session data
│   └── checkpoints/        # Checkpoint storage
├── your-project-files/     # Your actual project
└── examples/               # These examples
```

## 🤝 Contributing Examples

When adding new examples:

1. **Choose the right category** (task-inputs, configurations, references)
2. **Use descriptive filenames** that indicate the use case
3. **Add documentation** explaining the example's purpose
4. **Test the example** with actual ACA commands
5. **Update this README** with the new example

## 📚 Related Documentation

- [Main README](../README.md) - Project overview and installation
- [Design Documents](../docs/design/) - Architecture and system design
- [Session Documentation](../docs/sessions/) - Development session logs

## 🆘 Troubleshooting

### Common Issues

1. **File not found**: Ensure paths are correct relative to your working directory
2. **Permission errors**: Check that ACA has read access to example files
3. **TOML parsing errors**: Validate TOML syntax in configuration files
4. **Reference file resolution**: Referenced files must exist relative to the task file

### Getting Help

```bash
# Show CLI help
aca --help

# Show configuration discovery info
aca --show-config

# Run in verbose mode for debugging
aca --verbose [other options]
```