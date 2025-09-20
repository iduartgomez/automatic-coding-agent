//! # Claude Code Integration Layer
//!
//! Handles direct integration with Claude Code including rate limiting,
//! error recovery, context management, and usage tracking for efficient
//! and reliable LLM interactions.
//!
//! ## Core Components
//!
//! - **[`ClaudeCodeInterface`]**: Main interface for Claude Code interactions
//! - **[`RateLimiter`]**: Token bucket rate limiting with adaptive backoff
//! - **[`ContextManager`]**: Conversation context optimization and compression
//! - **[`ErrorRecoveryManager`]**: Circuit breaker and retry mechanisms
//! - **[`UsageTracker`]**: Cost tracking and performance analytics
//!
//! ## Key Features
//!
//! ### 🚦 Rate Limiting
//! - Token bucket algorithm for request and token rate limiting
//! - Adaptive exponential backoff with jitter
//! - Configurable burst allowance and backoff multipliers
//! - Automatic rate limit detection and handling
//!
//! ### 🧠 Context Management
//! - Intelligent conversation context optimization
//! - Automatic context compression when limits approached
//! - Relevance-based message filtering and summarization
//! - Configurable context window and history management
//!
//! ### 🛡️ Error Recovery
//! - Circuit breaker pattern for service protection
//! - Automatic retry with exponential backoff
//! - Graceful degradation on persistent failures
//! - Error classification and recovery strategies
//!
//! ### 📊 Usage Tracking
//! - Real-time token consumption monitoring
//! - Cost estimation and budget tracking
//! - Performance metrics and response time analytics
//! - Session-based usage aggregation
//!
//! ### 🔄 Session Management
//! - Connection pooling for efficient resource usage
//! - Session lifecycle management and cleanup
//! - Health monitoring and status reporting
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use automatic_coding_agent::claude::{ClaudeCodeInterface, ClaudeConfig, TaskRequest, TaskPriority};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Configure Claude interface
//!     let config = ClaudeConfig::default();
//!     let claude = ClaudeCodeInterface::new(config).await?;
//!
//!     // Create a task request
//!     let request = TaskRequest {
//!         id: uuid::Uuid::new_v4(),
//!         task_type: "code_generation".to_string(),
//!         description: "Create a REST API endpoint".to_string(),
//!         context: HashMap::new(),
//!         priority: TaskPriority::Normal,
//!         estimated_tokens: Some(1000),
//!     };
//!
//!     // Execute the task
//!     let response = claude.execute_task_request(request).await?;
//!     println!("Response: {}", response.response_text);
//!
//!     Ok(())
//! }
//! ```

/// Conversation context optimization and management.
///
/// Handles intelligent context compression, relevance filtering,
/// and conversation history optimization for efficient LLM interactions.
pub mod context_manager;

/// Error recovery and circuit breaker implementation.
///
/// Provides robust error handling with circuit breaker patterns,
/// automatic retries, and graceful degradation strategies.
pub mod error_recovery;

/// Main Claude Code interface and session management.
///
/// The primary interface for interacting with Claude Code, including
/// session pooling, request handling, and response processing.
pub mod interface;

/// Token bucket rate limiting with adaptive backoff.
///
/// Implements sophisticated rate limiting to stay within API quotas
/// while maximizing throughput and minimizing latency.
pub mod rate_limiter;

/// Core types and configuration structures.
///
/// Defines all data types, configuration structures, and enums
/// used throughout the Claude integration layer.
pub mod types;

/// Usage tracking and cost analytics.
///
/// Monitors token consumption, estimates costs, and provides
/// detailed analytics on Claude API usage patterns.
pub mod usage_tracker;

#[cfg(test)]
pub mod tests;

pub use context_manager::ContextManager;
pub use error_recovery::ErrorRecoveryManager;
pub use interface::ClaudeCodeInterface;
pub use rate_limiter::RateLimiter;
pub use types::*;
pub use usage_tracker::UsageTracker;
