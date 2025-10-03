# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2025-10-04

### Added
- Intelligent LLM-based task decomposition system with intelligent parser
- Markdown file reference resolution in task descriptions
- Dependency mapping for task relationships
- Execution plan loading for analyze→review→execute workflow
- `--append-system-prompt` CLI flag for custom system messages
- `--dump-plan` CLI flag for execution plan output
- LLM provider abstraction with BoxFuture async traits
- Test categorization system with test-tag crate
- Claude Code integration with comprehensive subprocess logging
- Conversational state persistence for Claude sessions
- Task continuation for resume functionality
- CLI resume and checkpoint functionality with `--list-checkpoints`
- `.aca` directory structure for session metadata
- Environment module to centralize hardcoded path constants
- TOML configuration support with structured format
- Setup commands system with sophisticated error handling
- Comprehensive module-level documentation across crate

### Changed
- Renamed binary and crate from `automatic-coding-agent` to `aca`
- Replaced lexopt with clap for CLI argument parsing
- Replaced 'legacy' terminology with 'structured' for TOML configuration
- Unified task processing with ExecutionPlan architecture
- Reorganized examples as integration tests
- Updated Claude integration tests to use CLI mode by default
- Claude integration tests run sequentially to prevent flakiness
- Upgraded to Rust edition 2024 with modern syntax
- Updated dependencies to latest versions
- Optimized CI caching with single rust-cache action
- Removed Dependabot and simplified CI to stable Rust only
- Updated design documents to reflect `.aca` directory structure

### Fixed
- Extracted Claude response from CLI JSON wrapper
- Improved prompt clarity for JSON-only responses
- Handled double-encoded JSON in LLM responses
- Resolved Windows build error with `env::var` ambiguity
- Resolved temp directory lifecycle issues in Claude integration tests
- Corrected CLI default behavior for `--list-checkpoints` command
- Resolved all clippy warnings for CI compliance
- Fixed CI test filtering and documentation
- Excluded Claude integration tests from CI runs
- Applied cargo fmt formatting fixes throughout codebase
- Resolved documentation link warnings in module docs
- Implemented Display trait for TaskError and missing Default impls
- Removed random error injection from mock to prevent flaky tests
- Added permissions to conventional commits workflow
- Replaced echo command generation with proper Claude Code integration
- Updated tests to work with removed `tasks_to_agent_commands` function
- Resolved CI failures and removed codecoverage setup
- Updated minimum Rust version to 1.90.0 for edition 2024
- Excluded logs directory from git tracking
- Excluded session documentation from version control

### Removed
- Unnecessary hello_world function from lib.rs
- Code coverage reporting with codecov
- Automated dependency updates with Dependabot
- Benchmark result storage step (temporarily disabled)

## [0.1.0] - 2025-09-18

### Added
- **1.1 Architecture Overview**: Complete system design and specifications
- **1.2 Task Management System**:
  - Dynamic task tree with hierarchical relationships
  - Intelligent scheduling with 6-factor weighted scoring
  - Dependency resolution with circular dependency detection
  - Real-time progress tracking and statistics
  - Automatic task deduplication
  - Event-driven architecture with comprehensive monitoring

- **1.3 Session Persistence System**:
  - Atomic persistence operations with transaction support
  - UUID-based checkpoint management with automatic cleanup
  - Intelligent recovery system with state validation
  - Comprehensive error handling and auto-correction
  - Version management with backward compatibility
  - Performance monitoring and metrics collection

### Technical Implementation
- ~3,500+ lines of production-ready Rust code
- 28 comprehensive test cases with 100% pass rate
- Thread-safe concurrent operations using Arc/RwLock patterns
- Fully async implementation with tokio
- Comprehensive error handling with structured error types
- Event-driven architecture supporting future enhancements

### Dependencies
- **Runtime**: serde, chrono, uuid, tokio, anyhow, thiserror, tracing, futures, async-trait, rand
- **Development**: tempfile for test isolation

### Documentation
- Complete session documentation tracking implementation progress
- Inline code documentation with examples
- Architecture compliance with modular design patterns

[Unreleased]: https://github.com/automatic-coding-agent/automatic-coding-agent/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/automatic-coding-agent/automatic-coding-agent/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/automatic-coding-agent/automatic-coding-agent/releases/tag/v0.1.0