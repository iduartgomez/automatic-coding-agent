//! # Hierarchical Task Management System
//!
//! Provides dynamic task trees, intelligent scheduling, dependency resolution,
//! and progress tracking for complex coding automation workflows.
//!
//! ## Core Components
//!
//! - **[`TaskManager`]**: Central orchestrator for task lifecycle and coordination
//! - **`TaskTree`**: Hierarchical task organization with parent-child relationships
//! - **`TaskScheduler`**: Intelligent prioritization with multi-factor scoring
//! - **`TaskExecutor`**: Resource allocation and execution environment management
//!
//! ## Key Features
//!
//! ### 🌳 Dynamic Task Tree
//! - Hierarchical task organization with unlimited nesting
//! - Automatic subtask creation during execution
//! - Parent-child relationship tracking and inheritance
//! - Context propagation through task hierarchy
//!
//! ### 🧠 Intelligent Scheduling
//! - Multi-factor weighted scoring system
//! - Resource-aware prioritization
//! - Deadline and complexity consideration
//! - Dynamic re-prioritization based on progress
//!
//! ### 🔗 Dependency Resolution
//! - Complex dependency tracking and validation
//! - Circular dependency detection and prevention
//! - Automatic execution ordering based on dependencies
//! - Parallel execution of independent tasks
//!
//! ### 📊 Progress Tracking
//! - Real-time task status monitoring
//! - Completion estimation and time tracking
//! - Resource usage analytics
//! - Performance metrics and statistics
//!
//! ### 🔄 Auto-Deduplication
//! - Automatic detection of similar tasks
//! - Intelligent merging of duplicate requests
//! - Conflict resolution for overlapping work
//!
//! ## Task Lifecycle
//!
//! ```text
//! Pending -> Scheduled -> InProgress -> [Completed | Failed | Blocked]
//!                    \-> Paused -----> InProgress
//! ```
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use aca::task::{TaskManager, TaskManagerConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = TaskManagerConfig::default();
//!     let task_manager = TaskManager::new(config);
//!
//!     // Task creation involves building TaskSpec instances
//!     // See the manager module documentation for detailed examples
//!
//!     Ok(())
//! }
//! ```

/// Task execution environment and resource management.
///
/// Handles the actual execution of tasks including resource allocation,
/// environment setup, and integration with LLM providers.
pub mod execution;

/// Central task lifecycle management and coordination.
///
/// The [`TaskManager`] orchestrates all task operations including
/// creation, scheduling, execution, and status tracking.
pub mod manager;

/// Intelligent task scheduling and prioritization.
///
/// Implements multi-factor scoring algorithms for optimal task
/// ordering and resource allocation.
pub mod scheduler;

/// Hierarchical task organization and relationship management.
///
/// Provides the [`TaskTree`] structure for organizing tasks in
/// parent-child relationships with dependency tracking.
pub mod tree;

/// Core task types, enums, and data structures.
///
/// Defines all fundamental types used throughout the task management
/// system including [`Task`], [`TaskStatus`], [`TaskPriority`], etc.
pub mod types;

/// Unified execution plan abstraction for task processing.
///
/// Provides the [`ExecutionPlan`] type that consolidates both simple
/// task lists and structured configurations into a common execution model.
pub mod execution_plan;

#[cfg(test)]
mod tests;

pub use execution::*;
pub use execution_plan::*;
pub use manager::*;
pub use scheduler::*;
pub use tree::*;
pub use types::*;
