# Task Input Examples

This directory contains examples of different task input formats supported by ACA.

## 📄 Single Task Files (`--task-file`)

### `single_task.md`
**Use case:** Complex feature implementation requiring detailed specification

A comprehensive example showing how to specify a complete authentication system implementation. This demonstrates:
- Detailed functional requirements
- Technical specifications
- Security considerations
- Testing requirements

**Command:** `aca --task-file examples/task-inputs/single_task.md`

## 📋 Task List Files (`--tasks`)

### `task_list.md`
**Use case:** Multiple related tasks with external references

Shows how to organize multiple tasks with different priority levels and reference external documentation. Features:
- Multiple task formats (checkbox, bullet, numbered)
- Task priorities and categories
- References to external files (`-> filename`)
- Completed task tracking (`[x]`)

**Command:** `aca --tasks examples/task-inputs/task_list.md`

## 💡 Tips for Task Inputs

### Single Task Best Practices
- Include comprehensive requirements
- Specify technical constraints
- Define clear acceptance criteria
- Add security/performance considerations

### Task List Best Practices
- Group related tasks logically
- Use consistent formatting
- Reference detailed specs in separate files
- Mark completed tasks to track progress

### Reference File Syntax
Use the `->` syntax to include external files:
```markdown
- [ ] Fix authentication bug -> ../references/auth_analysis.md
```

The referenced file content is automatically included when processing the task.