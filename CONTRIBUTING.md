# Contributing to Automatic Coding Agent

Thank you for your interest in contributing to the Automatic Coding Agent project! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Code Style](#code-style)
- [Submitting Changes](#submitting-changes)
- [Project Structure](#project-structure)

## Code of Conduct

This project adheres to a code of conduct adapted from the [Contributor Covenant](https://www.contributor-covenant.org/). By participating, you are expected to uphold this code.

## Getting Started

### Prerequisites

- Rust 1.75.0 or later
- Git
- A GitHub account

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/automatic-coding-agent.git
   cd automatic-coding-agent
   ```

## Development Setup

### Install Development Tools

```bash
# Install useful development tools
cargo install cargo-audit cargo-llvm-cov cargo-nextest

# Install pre-commit hooks (optional but recommended)
pip install pre-commit
pre-commit install
```

### Build and Test

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run tests with coverage
cargo llvm-cov --open

# Check code quality
cargo clippy --all-targets --all-features
cargo fmt --all --check

# Security audit
cargo audit
```

## Making Changes

### Branching Strategy

- Create feature branches from `master`
- Use descriptive branch names: `feature/task-scheduling-improvements`, `fix/session-recovery-bug`
- Keep branches focused on a single feature or fix

### Commit Messages

We strictly follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Required Types:**
- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes
- `perf`: Performance improvements
- `build`: Build system changes

**Optional Scopes:**
- `task`: Task management system
- `session`: Session persistence system
- `cli`: Command line interface
- `docker`: Docker-related changes
- `ci`: CI/CD workflows

**Breaking Changes:**
Use `!` after type/scope for breaking changes:
```
feat(task)!: change task API to support new scheduling algorithm

BREAKING CHANGE: The TaskSpec structure now requires a priority field
```

**Examples:**
```
feat(task): add intelligent task scheduling with weighted scoring

Implements a 6-factor weighted scoring system for task prioritization
including priority, dependencies, context similarity, and resource
availability.

Closes #123

feat(session): implement atomic persistence with transaction support

- Add PersistenceManager with atomic operations
- Implement rollback capability for failed transactions
- Add comprehensive error handling and validation

ci: add comprehensive GitHub Actions workflow

- Multi-platform testing across Linux/Windows/macOS
- Code quality checks with clippy and formatting
- Security auditing and coverage reporting

fix(scheduler): resolve deadlock in task selection algorithm

The task selection was causing deadlocks when multiple tasks had
circular dependencies. This fix implements proper dependency resolution
with cycle detection.

Fixes #456
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test session::tests

# Run tests with output
cargo test -- --nocapture

# Run only integration tests
cargo test --test integration
```

### Writing Tests

- Add unit tests in the same file as the code being tested
- Use the `#[cfg(test)]` module for test organization
- Write integration tests in the `tests/` directory
- Include both positive and negative test cases
- Test edge cases and error conditions

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature_functionality() {
        // Arrange
        let input = create_test_input();

        // Act
        let result = function_under_test(input).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().field, expected_value);
    }
}
```

## Code Style

### Rust Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for consistent formatting
- Address all `cargo clippy` warnings
- Prefer explicit error handling over panics
- Use meaningful variable and function names
- Add documentation comments for public APIs

### Documentation

- Document all public APIs with `///` comments
- Include examples in documentation when helpful
- Update README.md for significant changes
- Maintain session logs in `docs/sessions/` for major features

Example documentation:
```rust
/// Creates a new task with the specified configuration.
///
/// # Arguments
///
/// * `spec` - The task specification containing title, description, and metadata
/// * `parent_id` - Optional parent task ID for creating subtasks
///
/// # Returns
///
/// Returns the ID of the newly created task.
///
/// # Examples
///
/// ```rust
/// # use automatic_coding_agent::*;
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let task_spec = TaskSpec {
///     title: "Implement feature".to_string(),
///     description: "Add new functionality".to_string(),
///     // ... other fields
/// };
///
/// let task_id = manager.create_task(task_spec, None).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_task(&self, spec: TaskSpec, parent_id: Option<TaskId>) -> Result<TaskId> {
    // Implementation
}
```

## Submitting Changes

### Before Submitting

1. Ensure all tests pass: `cargo test`
2. Check code formatting: `cargo fmt --all --check`
3. Address clippy warnings: `cargo clippy --all-targets --all-features`
4. Run security audit: `cargo audit`
5. Update documentation if needed
6. Add relevant tests for your changes

### Pull Request Process

1. **Create a Pull Request** from your feature branch to `master`
2. **Fill out the PR template** with:
   - Clear description of changes
   - Type of change (bug fix, feature, etc.)
   - Testing performed
   - Breaking changes (if any)
3. **Ensure CI passes** - all GitHub Actions workflows must pass
4. **Request review** from maintainers
5. **Address feedback** and update your PR as needed

### PR Guidelines

- Keep PRs focused and reasonably sized
- Include tests for new functionality
- Update documentation for API changes
- Rebase on master before submitting (if needed)
- Respond promptly to review feedback

## Project Structure

```
automatic-coding-agent/
├── .github/              # GitHub workflows and templates
├── docs/                 # Documentation
│   ├── design/          # Architecture and design documents
│   └── sessions/        # Development session logs
├── src/                 # Source code
│   ├── session/         # Session persistence system
│   ├── task/           # Task management system
│   ├── lib.rs          # Library root
│   └── main.rs         # Binary entry point
├── tests/              # Integration tests
├── Cargo.toml          # Project configuration
└── README.md           # Project overview
```

### Key Components

- **Task Management** (`src/task/`): Dynamic task trees, scheduling, execution
- **Session Persistence** (`src/session/`): State management, checkpoints, recovery
- **Core Types** (`src/task/types.rs`): Fundamental data structures and types

## Getting Help

- **Issues**: Use GitHub issues for bug reports and feature requests
- **Discussions**: Use GitHub discussions for questions and general discussion
- **Documentation**: Check the docs/ directory and generated API docs

## Recognition

Contributors will be recognized in:
- CHANGELOG.md for significant contributions
- GitHub contributors page
- Special thanks in release notes for major features

Thank you for contributing to make the Automatic Coding Agent better!