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
//! - **[`session`]**: Complete session lifecycle management with atomic persistence
//! - **[`task`]**: Hierarchical task management with intelligent scheduling
//! - **[`claude`]**: Claude Code integration with rate limiting and error recovery
//! - **[`llm`]**: Provider-agnostic LLM interface supporting multiple providers
//! - **[`integration`]**: High-level system orchestration and agent coordination
//!
//! ## Features
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
//! ### 🤖 LLM Provider Abstraction
//! - **Multi-Provider Support**: Claude, OpenAI, Anthropic API, local models (Ollama)
//! - **Provider-Agnostic Interface**: Unified API across all LLM providers
//! - **Automatic Fallback**: Seamless fallback between providers for reliability
//! - **Rate Limiting**: Provider-specific rate limiting and cost optimization
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

// Re-export main session types
pub use session::{SessionInitOptions, SessionManager, SessionManagerConfig, SessionMetadata};

// Re-export main task types
pub use task::{Task, TaskManager, TaskManagerConfig, TaskPriority, TaskSpec, TaskStatus};

// Re-export main Claude types
pub use claude::{ClaudeCodeInterface, ClaudeConfig};

// Re-export LLM abstraction types
pub use llm::{LLMProvider, LLMRequest, LLMResponse, ProviderConfig, ProviderType};

// Re-export integration types
pub use integration::{AgentConfig, AgentSystem, SystemStatus};

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
pub fn hello_world() {
    println!("Hello, World!");
}
