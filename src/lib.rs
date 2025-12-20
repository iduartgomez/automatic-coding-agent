//! # Automatic Coding Agent
//!
//! A Rust-based agentic tool that automates coding tasks using multiple LLM providers.
//! The system operates with dynamic task trees, comprehensive session persistence,
//! and full resumability for long-running automated coding sessions.
//!
//! ## Architecture Overview
//!
//! The system consists of several key components organized into modules:
//!
//! - **[`cli`]**: Command-line interface with intelligent task parsing and simple task loading
//! - **[`llm`]**: Provider-agnostic LLM interface supporting multiple providers (Claude Code CLI/API, OpenAI Codex CLI, etc.)
//! - **[`claude`]**: Claude Code integration with rate limiting and error recovery
//! - **[`task`]**: Hierarchical task management with intelligent scheduling
//! - **[`session`]**: Complete session lifecycle management with atomic persistence
//! - **[`integration`]**: High-level system orchestration and agent coordination
//!
//! ## Features
//!
//! ### 🤖 Intelligent Task Parsing
//! - **LLM-based decomposition**: Analyzes complex tasks and breaks them into structured hierarchies
//! - **Markdown file resolution**: Automatically follows and includes referenced files
//! - **Detail preservation**: Expands 6 high-level tasks into 42+ detailed subtasks
//! - **Dependency mapping**: Automatic TaskId generation and dependency graph construction
//!
//! ### 🔌 LLM Provider System
//! - **Multi-Provider Support**: Claude Code CLI (default), Claude API, OpenAI Codex CLI, local models (Ollama)
//! - **CLI Mode (default)**: Uses `claude` command, no API key required
//! - **API Mode**: Direct Anthropic API access with API key
//! - **Provider-Agnostic Interface**: Unified API across all LLM providers
//! - **Rate Limiting**: Provider-specific rate limiting and cost optimization
//!
//! ### 🎯 Task Management
//! - **Dynamic Task Tree**: Hierarchical task organization with parent-child relationships
//! - **Intelligent Scheduling**: Multi-factor scoring system with resource-aware prioritization
//! - **Dependency Resolution**: Complex dependency tracking with circular dependency detection
//! - **Progress Tracking**: Real-time statistics and completion estimation
//!
//! ### 💾 Session Persistence
//! - **Atomic Operations**: Thread-safe persistence with transaction support and rollback
//! - **Checkpoint System**: UUID-based checkpoint creation with automatic cleanup
//! - **Recovery Manager**: Intelligent recovery from corruption and failures
//! - **State Validation**: Comprehensive integrity checking with auto-correction
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use aca::{AgentSystem, AgentConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize the agent system
//!     let config = AgentConfig::default();
//!     let agent = AgentSystem::new(config).await?;
//!
//!     // Create and process a task
//!     let task_id = agent.create_and_process_task(
//!         "Implement feature",
//!         "Add new functionality to the codebase"
//!     ).await?;
//!
//!     println!("Task completed: {}", task_id);
//!     Ok(())
//! }
//! ```

/// Session management and persistence functionality.
///
/// This module provides comprehensive session lifecycle management including
/// atomic persistence, checkpoint creation, and intelligent recovery capabilities.
pub mod session;

/// Hierarchical task management system.
///
/// Provides dynamic task trees, intelligent scheduling, dependency resolution,
/// and progress tracking for complex coding automation workflows.
pub mod task;

/// Claude Code integration layer.
///
/// Handles direct integration with Claude Code including rate limiting,
/// error recovery, context management, and usage tracking.
pub mod claude;

/// OpenAI Codex integration layer.
///
/// Executes the Codex CLI headlessly with session logging and rate limiting.
pub mod openai;

/// Provider-agnostic LLM interface.
///
/// Abstraction layer supporting multiple LLM providers (Claude, OpenAI, local models)
/// with unified API, automatic fallback, and provider-specific optimizations.
pub mod llm;

/// High-level system integration and orchestration.
///
/// Combines all subsystems into a cohesive agent architecture with
/// coordinated task processing and system-wide status monitoring.
pub mod integration;

/// Environment constants and path utilities.
///
/// Centralizes all hardcoded paths and directory names used throughout
/// the application for easier maintenance and consistency.
pub mod env;

/// Container orchestration and isolation layer.
///
/// Provides containerized execution environments for task isolation
/// using Docker/Podman via the bollard API. Handles container lifecycle
/// management, resource limits, networking, volume mounting, and monitoring.
///
/// This module is only available with the `containers` feature enabled.
#[cfg(feature = "containers")]
pub mod container;

/// Command execution abstraction layer.
///
/// Provides a unified interface for executing commands either on the host
/// system or within containers, enabling transparent sandboxed execution.
/// Includes `HostExecutor` for native execution and `ContainerExecutor`
/// for containerized execution (requires `containers` feature).
pub mod executor;

// Re-export main session types
pub use session::{SessionInitOptions, SessionManager, SessionManagerConfig, SessionMetadata};

// Re-export main task types
pub use task::{Task, TaskManager, TaskManagerConfig, TaskPriority, TaskSpec, TaskStatus};

// Re-export main Claude types
pub use claude::{ClaudeCodeInterface, ClaudeConfig};

// Re-export OpenAI types
pub use openai::{OpenAICodexInterface, OpenAIConfig};

// Re-export LLM abstraction types
pub use llm::{LLMProvider, LLMRequest, LLMResponse, ProviderConfig, ProviderType};

// Re-export integration types
pub use integration::{AgentConfig, AgentSystem, SystemStatus};

// Re-export container types (only with containers feature)
#[cfg(feature = "containers")]
pub use container::{
    ACA_BASE_IMAGE, ACA_BASE_IMAGE_ALPINE, ContainerClient, ContainerConfig, ContainerError,
    ContainerOrchestrator, ExecConfig, ExecOutput, ImageBuilder,
};

// Re-export executor types
pub use executor::{
    CommandExecutor, ContainerExecutionConfig, ExecutionCommand, ExecutionMode, ExecutionResult,
    ExecutorError, HostExecutor, ResourceAllocation, SystemResources,
};

#[cfg(feature = "containers")]
pub use executor::ContainerExecutor;

// CLI module for command-line interface
pub mod cli;

/// Prints "Hello, World!" to stdout.
///
/// # Examples
///
/// ```
/// use aca::hello_world;
///
/// hello_world();
/// ```
#[allow(dead_code)]
pub fn hello_world() {
    println!("Hello, World!");
}
