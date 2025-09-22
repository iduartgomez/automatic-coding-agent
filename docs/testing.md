# Testing Guide

This document explains the testing strategy and how to run different types of tests in the automatic-coding-agent project.

## Test Categories

The project uses test categories to organize tests based on their dependencies and requirements:

### 1. Unit Tests

Standard unit tests that don't require external dependencies. These test individual functions and modules in isolation.

**Location**: Throughout the codebase in `#[cfg(test)]` modules
**Dependencies**: None
**Run with**: `cargo test --lib`

### 2. Integration Tests (Non-Claude)

Integration tests that test system components working together but don't require the Claude CLI or external API access.

**Location**: `tests/` directory
**Examples**:
- `config_generation_integration.rs` - Configuration parsing and generation
- `cli_functionality.rs` - Task parsing and CLI components
- `config_toml_integration.rs` - TOML configuration handling

**Dependencies**: None
**Run with**: `cargo test -- :!claude:`

### 3. Claude Integration Tests

Integration tests that require the Claude CLI to be installed and available. These tests are tagged with `#[tag(claude)]` and excluded from CI by default.

**Location**: `tests/` directory
**Examples**:
- `claude_integration.rs` - Full Claude Code integration testing
- `setup_commands_integration.rs` - AgentSystem initialization
- `error_handling_integration.rs` - Error handling with AgentSystem
- `backup_strategy_integration.rs` - Backup strategies with AgentSystem

**Dependencies**:
- Claude CLI installed (`claude` command available)
- Valid Claude API access
- Network connectivity

**Run with**: `cargo test -- :claude:`

## Running Tests

### All Tests (Excluding Claude Integration)

This is the default mode for CI and regular development:

```bash
# Run all unit and integration tests (excluding Claude)
cargo test --lib && cargo test --test cli_functionality --test config_generation_integration --test config_toml_integration

# With verbose output
cargo test --lib --nocapture && cargo test --test cli_functionality --test config_generation_integration --test config_toml_integration --nocapture

# With all features
cargo test --all-features --lib && cargo test --all-features --test cli_functionality --test config_generation_integration --test config_toml_integration
```

### Claude Integration Tests Only

To run only the Claude integration tests:

```bash
# Run only Claude integration tests
cargo test --test setup_commands_integration --test backup_strategy_integration --test error_handling_integration --test claude_integration

# With verbose output
cargo test --test setup_commands_integration --test backup_strategy_integration --test error_handling_integration --test claude_integration --nocapture
```

### Specific Test Categories

```bash
# Run only unit tests
cargo test --lib

# Run only integration tests (non-Claude)
cargo test --test config_generation_integration
cargo test --test cli_functionality

# Run specific Claude integration test
cargo test --test claude_integration
```

### Running Individual Tests

```bash
# Run a specific test function
cargo test test_simple_file_creation

# Run all tests matching a pattern
cargo test task_parsing
```

## Test Environment Setup

### For Non-Claude Tests

No special setup required. These tests use temporary directories and mock data.

### For Claude Integration Tests

1. **Install Claude CLI**:
   ```bash
   # Follow Claude CLI installation instructions
   # Ensure 'claude' command is available in PATH
   claude --version
   ```

2. **Authentication**:
   Make sure you're authenticated with Claude:
   ```bash
   claude auth login
   ```

3. **Environment Variables** (optional):
   ```bash
   export RUST_LOG=info  # Enable detailed logging
   ```

## Continuous Integration

### Standard CI Pipeline

The main CI pipeline runs on every push and pull request:

- ✅ Unit tests
- ✅ Integration tests (non-Claude)
- ✅ Linting (rustfmt, clippy)
- ✅ Security audit
- ✅ Build checks (multiple platforms)
- ❌ Claude integration tests (excluded)

### Claude Integration CI

Claude integration tests are excluded from the standard CI pipeline because they require:
- External API access
- Claude CLI installation
- Authentication credentials

**Manual Trigger**: Claude tests can be manually triggered in CI by:

1. **Workflow Dispatch**: Go to GitHub Actions → CI → "Run workflow"
2. **Commit Message**: Include `[run-claude-tests]` in your commit message

**Example**:
```bash
git commit -m "feat: improve Claude integration [run-claude-tests]"
```

## Test Structure

### Test File Organization

```
tests/
├── resources/                    # Test data and resources
│   ├── test1-simple-file/       # Simple file creation test setup
│   ├── test2-readme-creation/   # README generation test setup
│   ├── test3-file-editing/      # File editing test setup
│   ├── test4-multi-task/        # Multi-task execution test setup
│   └── test5-task-references/   # Task reference test setup
├── cli_functionality.rs         # [Non-Claude] CLI component tests
├── config_generation_integration.rs  # [Non-Claude] Config tests
├── config_toml_integration.rs   # [Non-Claude] TOML config tests
├── claude_integration.rs        # [Claude] Full Claude integration
├── setup_commands_integration.rs # [Claude] Setup command tests
├── error_handling_integration.rs # [Claude] Error handling tests
└── backup_strategy_integration.rs # [Claude] Backup strategy tests
```

### Test Tagging

Tests are tagged using the `test-tag` crate:

```rust
use test_tag::tag;

#[tokio::test]
#[tag(claude)]  // Requires Claude CLI
async fn test_claude_functionality() {
    // Test implementation
}

#[test]
// No tag = regular unit/integration test
fn test_config_parsing() {
    // Test implementation
}
```

## Writing New Tests

### Adding Unit Tests

Add tests directly in the source files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        // Test implementation
    }
}
```

### Adding Integration Tests (Non-Claude)

Create or update files in `tests/` without the `#[tag(claude)]` attribute:

```rust
#[test]
fn test_integration_functionality() {
    // Test that doesn't need Claude
}
```

### Adding Claude Integration Tests

Create or update files in `tests/` with the `#[tag(claude)]` attribute:

```rust
use test_tag::tag;

#[tokio::test]
#[tag(claude)]
async fn test_claude_integration() {
    // Test that requires Claude CLI
}
```

## Test Data and Resources

### Test Resources Directory

The `tests/resources/` directory contains only setup files and templates. Generated files are created in temporary directories during test execution and automatically cleaned up.

**Structure**:
- Each test scenario has its own subdirectory
- Only original task files and reference materials are stored
- No generated outputs, session files, or temporary data

### Using Test Resources

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn test_with_resources() {
    // Create isolated temporary workspace
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Copy test resources to temp workspace
    let source = "tests/resources/test1-simple-file/task.md";
    let dest = workspace.join("task.md");
    fs::copy(source, dest).unwrap();

    // Run test with isolated workspace
    // Temp directory automatically cleaned up
}
```

## Debugging Tests

### Verbose Output

```bash
# Show detailed test output
cargo test -- --nocapture

# Show debug logs
RUST_LOG=debug cargo test
```

### Test Logs

Claude integration tests create log files in the workspace:

```
logs/
├── claude-subprocess-{task-id}.log  # Subprocess execution logs
└── session.log                     # Session activity logs
```

### Failed Test Investigation

```bash
# Run a specific failing test
cargo test test_name -- --exact --nocapture

# Run with debug logging
RUST_LOG=debug cargo test test_name -- --exact --nocapture
```

## Performance Considerations

### Test Isolation

- Each test uses isolated temporary directories
- No shared state between tests
- Parallel execution is safe

### Resource Usage

- Claude integration tests may take longer due to API calls
- Network dependencies can cause intermittent failures
- Consider timeout settings for Claude API calls

### CI Optimization

- Non-Claude tests run in parallel
- Caching is used for dependencies and build artifacts
- Claude tests are separated to avoid CI timeouts

## Troubleshooting

### Common Issues

**Tests fail with "Claude CLI not found"**:
- Install Claude CLI: Follow installation instructions
- Check PATH: `which claude`
- Verify installation: `claude --version`

**Authentication errors**:
- Login to Claude: `claude auth login`
- Check credentials: `claude auth status`

**Permission errors in tests**:
- Check file permissions in test resources
- Ensure temporary directories are writable
- Verify workspace directory permissions

**Network timeout errors**:
- Check internet connectivity
- Verify Claude API access
- Consider increasing timeout values

### Reporting Test Issues

When reporting test failures, include:

1. Test command used
2. Full error output
3. Environment details (OS, Rust version, Claude CLI version)
4. Whether it's reproducible
5. Any relevant log files

## Best Practices

### Test Design

- Keep tests focused and atomic
- Use descriptive test names
- Include both positive and negative test cases
- Test error conditions and edge cases

### Resource Management

- Always use temporary directories for test workspaces
- Clean up resources in test teardown
- Don't commit generated test files to the repository

### CI Considerations

- Design tests to be deterministic
- Avoid tests that depend on external services in CI
- Use appropriate timeouts for async operations
- Consider test execution time and resource usage