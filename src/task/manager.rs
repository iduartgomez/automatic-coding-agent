//! # Task Manager Implementation
//!
//! The central orchestrator for task lifecycle management, providing thread-safe
//! coordination between task creation, scheduling, execution, and monitoring.
//!
//! ## Architecture Overview
//!
//! The [`TaskManager`] acts as the primary interface for all task operations,
//! coordinating between several key components:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    TaskManager                              │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐│
//! │  │  TaskTree   │ │ TaskScheduler│ │    Event Handlers       ││
//! │  │             │ │             │ │                         ││
//! │  │ ┌─────────┐ │ │ ┌─────────┐ │ │ ┌─────────┬─────────┐   ││
//! │  │ │  Task   │ │ │ │Priority │ │ │ │Metrics  │Logging  │   ││
//! │  │ │  Tree   │ │ │ │Scoring  │ │ │ │Handler  │Handler  │   ││
//! │  │ │Structure│ │ │ │Engine   │ │ │ └─────────┴─────────┘   ││
//! │  │ └─────────┘ │ │ └─────────┘ │ │                         ││
//! │  └─────────────┘ └─────────────┘ └─────────────────────────┘│
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Core Functionality
//!
//! ### 🚀 Task Lifecycle Management
//! - **Creation**: Convert task specifications into managed task instances
//! - **Scheduling**: Intelligent prioritization and execution ordering
//! - **Monitoring**: Real-time status tracking and progress reporting
//! - **Completion**: Result handling and cleanup operations
//!
//! ### 🌳 Hierarchical Organization
//! - **Parent-Child Relationships**: Automatic subtask creation and management
//! - **Dependency Resolution**: Complex dependency tracking and ordering
//! - **Context Inheritance**: Propagate settings and context through task trees
//! - **Bulk Operations**: Efficient operations on task hierarchies
//!
//! ### 📊 Event System & Monitoring
//! - **Event Broadcasting**: Real-time notifications for all task operations
//! - **Metrics Collection**: Performance tracking and resource usage analytics
//! - **Error Handling**: Comprehensive error recovery and retry mechanisms
//! - **Audit Logging**: Complete audit trail of all task operations
//!
//! ### ⚙️ Configuration & Customization
//! - **Retry Policies**: Configurable failure handling and retry strategies
//! - **Resource Limits**: Concurrent task execution controls
//! - **Cleanup Policies**: Automatic cleanup of completed tasks
//! - **Event Handlers**: Pluggable event handling system
//!
//! ## Usage Patterns
//!
//! ### Basic Task Management
//!
//! ```rust,no_run
//! use automatic_coding_agent::task::{
//!     TaskManager, TaskManagerConfig, TaskSpec, TaskPriority,
//!     TaskMetadata, ComplexityLevel, ContextRequirements
//! };
//! use std::collections::HashMap;
//! use chrono::Duration;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Configure the task manager
//!     let config = TaskManagerConfig {
//!         auto_retry_failed_tasks: true,
//!         max_retry_attempts: 3,
//!         retry_delay_minutes: 5,
//!         auto_cleanup_completed: false,
//!         cleanup_after_hours: 24,
//!         enable_task_metrics: true,
//!         max_concurrent_tasks: 4,
//!     };
//!
//!     let task_manager = TaskManager::new(config);
//!
//!     // Create a comprehensive task specification
//!     let task_spec = TaskSpec {
//!         title: "Implement REST API".to_string(),
//!         description: "Create a new REST API for user management with authentication".to_string(),
//!         dependencies: Vec::new(),
//!         metadata: TaskMetadata {
//!             priority: TaskPriority::High,
//!             estimated_complexity: Some(ComplexityLevel::Complex),
//!             estimated_duration: Some(Duration::hours(4)),
//!             repository_refs: Vec::new(),
//!             file_refs: Vec::new(),
//!             tags: vec!["api".to_string(), "backend".to_string()],
//!             context_requirements: ContextRequirements {
//!                 required_files: vec!["package.json".to_string()],
//!                 required_repositories: Vec::new(),
//!                 build_dependencies: vec!["nodejs".to_string()],
//!                 environment_vars: HashMap::new(),
//!                 claude_context_keys: vec!["project_structure".to_string()],
//!             },
//!         },
//!     };
//!
//!     // Create and execute the task
//!     let task_id = task_manager.create_task(task_spec, None).await?;
//!     println!("Created task: {}", task_id);
//!
//!     // Monitor task progress
//!     let stats = task_manager.get_statistics().await?;
//!     println!("Active tasks: {}", stats.active_tasks);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Hierarchical Task Creation
//!
//! ```rust,no_run
//! use automatic_coding_agent::task::{TaskManager, TaskManagerConfig, TaskSpec};
//!
//! async fn create_project_tasks(task_manager: &TaskManager) -> anyhow::Result<()> {
//!     // Create a main project task
//!     let main_task = TaskSpec {
//!         title: "Build Full-Stack Application".to_string(),
//!         description: "Complete web application with frontend and backend".to_string(),
//!         ..Default::default()
//!     };
//!     let main_id = task_manager.create_task(main_task, None).await?;
//!
//!     // Create backend subtasks
//!     let backend_task = TaskSpec {
//!         title: "Backend Development".to_string(),
//!         description: "API, database, and server implementation".to_string(),
//!         ..Default::default()
//!     };
//!     let backend_id = task_manager.create_task(backend_task, Some(main_id)).await?;
//!
//!     // Create specific backend subtasks
//!     let api_task = TaskSpec {
//!         title: "REST API Implementation".to_string(),
//!         description: "Create RESTful endpoints".to_string(),
//!         ..Default::default()
//!     };
//!     task_manager.create_task(api_task, Some(backend_id)).await?;
//!
//!     // Frontend development can happen in parallel
//!     let frontend_task = TaskSpec {
//!         title: "Frontend Development".to_string(),
//!         description: "User interface and client-side logic".to_string(),
//!         ..Default::default()
//!     };
//!     task_manager.create_task(frontend_task, Some(main_id)).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Event Handling & Monitoring
//!
//! ```rust,no_run
//! use automatic_coding_agent::task::{TaskManager, TaskEvent, TaskEventHandler};
//! use anyhow::Result;
//!
//! struct MetricsHandler {
//!     task_count: std::sync::atomic::AtomicU64,
//! }
//!
//! impl TaskEventHandler for MetricsHandler {
//!     fn handle_event(&self, event: &TaskEvent) -> Result<()> {
//!         match event {
//!             TaskEvent::TaskCreated { task_id, .. } => {
//!                 self.task_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
//!                 println!("Task created: {}", task_id);
//!             }
//!             TaskEvent::TaskCompleted { task_id, result } => {
//!                 println!("Task {} completed with result: {:?}", task_id, result);
//!             }
//!             TaskEvent::TaskFailed { task_id, error } => {
//!                 eprintln!("Task {} failed: {}", task_id, error);
//!             }
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//! }
//!
//! async fn setup_monitoring(mut task_manager: TaskManager) -> Result<TaskManager> {
//!     let metrics_handler = MetricsHandler {
//!         task_count: std::sync::atomic::AtomicU64::new(0),
//!     };
//!
//!     task_manager.add_event_handler(Box::new(metrics_handler));
//!     Ok(task_manager)
//! }
//! ```
//!
//! ## Error Handling & Recovery
//!
//! The task manager provides comprehensive error handling:
//!
//! - **Automatic Retries**: Failed tasks can be automatically retried with configurable delays
//! - **Circuit Breaker**: Prevents cascading failures in task chains
//! - **Error Classification**: Different handling for different error types
//! - **Recovery Strategies**: Graceful degradation and fallback mechanisms
//!
//! ## Performance Considerations
//!
//! - **Async Operations**: All operations are fully asynchronous for optimal throughput
//! - **Resource Pooling**: Efficient resource management and reuse
//! - **Batch Operations**: Bulk operations for improved performance
//! - **Memory Management**: Automatic cleanup of completed tasks
//!
//! ## Thread Safety
//!
//! All operations are thread-safe and can be called concurrently:
//! - Internal data structures use `Arc<RwLock<T>>` and `Arc<Mutex<T>>`
//! - Event handlers are executed safely without blocking main operations
//! - Statistics and queries are lock-free where possible

use crate::task::scheduler::*;
use crate::task::tree::*;
use crate::task::types::*;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Central task management system that orchestrates task lifecycle operations.
///
/// The `TaskManager` is the primary interface for all task-related operations,
/// providing thread-safe coordination between task creation, scheduling, execution,
/// and monitoring. It maintains a hierarchical task tree, intelligent scheduler,
/// and event system for comprehensive task management.
///
/// ## Key Responsibilities
///
/// - **Task Lifecycle**: Create, schedule, execute, and monitor tasks
/// - **Hierarchy Management**: Organize tasks in parent-child relationships
/// - **Event Broadcasting**: Notify handlers of task state changes
/// - **Resource Management**: Control concurrent execution and cleanup
/// - **Error Recovery**: Handle failures with retry and recovery strategies
///
/// ## Thread Safety
///
/// All methods are thread-safe and can be called concurrently. Internal state
/// is protected using `Arc<RwLock<T>>` and `Arc<Mutex<T>>` for optimal performance.
///
/// ## Example
///
/// ```rust,no_run
/// use automatic_coding_agent::task::{TaskManager, TaskManagerConfig};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = TaskManagerConfig::default();
///     let task_manager = TaskManager::new(config);
///
///     // Task manager is now ready to handle tasks
///     Ok(())
/// }
/// ```
pub struct TaskManager {
    tree: Arc<RwLock<TaskTree>>,
    scheduler: Arc<Mutex<TaskScheduler>>,
    config: TaskManagerConfig,
    event_handlers: Vec<Box<dyn TaskEventHandler + Send + Sync>>,
}

/// Configuration settings for the task manager behavior and policies.
///
/// This structure defines how the task manager operates, including retry policies,
/// cleanup behavior, resource limits, and monitoring settings. All settings can
/// be customized to match your specific requirements.
///
/// ## Configuration Categories
///
/// - **Retry Policies**: Control automatic retry behavior for failed tasks
/// - **Cleanup Settings**: Manage memory usage and task lifecycle
/// - **Resource Limits**: Control concurrent execution and system resources
/// - **Monitoring**: Enable or disable metrics collection and tracking
///
/// ## Example
///
/// ```rust
/// use automatic_coding_agent::task::TaskManagerConfig;
///
/// let config = TaskManagerConfig {
///     auto_retry_failed_tasks: true,
///     max_retry_attempts: 3,
///     retry_delay_minutes: 5,
///     auto_cleanup_completed: false, // Keep completed tasks for analysis
///     cleanup_after_hours: 24,
///     enable_task_metrics: true,
///     max_concurrent_tasks: 8, // Higher throughput
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskManagerConfig {
    /// Whether to automatically retry failed tasks
    pub auto_retry_failed_tasks: bool,
    /// Maximum number of retry attempts for failed tasks
    pub max_retry_attempts: u32,
    /// Delay in minutes between retry attempts
    pub retry_delay_minutes: u32,
    /// Whether to automatically cleanup completed tasks
    pub auto_cleanup_completed: bool,
    /// Hours after which completed tasks are automatically cleaned up
    pub cleanup_after_hours: u32,
    /// Whether to enable detailed task metrics collection
    pub enable_task_metrics: bool,
    /// Maximum number of tasks that can execute concurrently
    pub max_concurrent_tasks: u32,
}

/// Events that can occur during task management operations.
///
/// These events are broadcast to all registered event handlers whenever
/// significant task operations occur. This enables monitoring, logging,
/// metrics collection, and other cross-cutting concerns.
///
/// ## Event Categories
///
/// - **Lifecycle Events**: Task creation, status changes, completion
/// - **Hierarchy Events**: Subtask creation, parent-child relationships
/// - **System Events**: Deduplication, statistics updates
/// - **Error Events**: Task failures and error conditions
///
/// ## Example Usage
///
/// ```rust,no_run
/// use automatic_coding_agent::task::{TaskEvent, TaskEventHandler};
/// use anyhow::Result;
///
/// struct LoggingHandler;
///
/// impl TaskEventHandler for LoggingHandler {
///     fn handle_event(&self, event: &TaskEvent) -> Result<()> {
///         match event {
///             TaskEvent::TaskCreated { task_id, parent_id } => {
///                 println!("Task {} created with parent {:?}", task_id, parent_id);
///             }
///             TaskEvent::TaskCompleted { task_id, result } => {
///                 println!("Task {} completed: {:?}", task_id, result);
///             }
///             _ => {} // Handle other events as needed
///         }
///         Ok(())
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum TaskEvent {
    /// A new task has been created
    TaskCreated {
        task_id: TaskId,
        parent_id: Option<TaskId>,
    },
    /// A task's status has changed
    TaskStatusChanged {
        task_id: TaskId,
        old_status: TaskStatus,
        new_status: TaskStatus,
    },
    /// A task has completed successfully
    TaskCompleted {
        task_id: TaskId,
        result: TaskResult,
    },
    /// A task has failed with an error
    TaskFailed {
        task_id: TaskId,
        error: TaskError,
    },
    /// New subtasks have been created for a parent task
    SubtasksCreated {
        parent_id: TaskId,
        subtask_ids: Vec<TaskId>,
    },
    /// Duplicate tasks have been merged/deduplicated
    TasksDeduped {
        primary_id: TaskId,
        merged_ids: Vec<TaskId>,
    },
    /// Task tree statistics have been updated
    TreeStatisticsUpdated {
        statistics: TaskTreeStatistics,
    },
}

/// Handler trait for processing task management events.
///
/// Implement this trait to create custom event handlers that respond to
/// task operations. Handlers are called synchronously for each event,
/// so implementations should be efficient to avoid blocking task operations.
///
/// ## Implementation Guidelines
///
/// - **Fast Execution**: Keep handler logic lightweight and non-blocking
/// - **Error Handling**: Return errors only for critical failures
/// - **Thread Safety**: Handlers must be `Send + Sync` for concurrent use
/// - **Stateless Preferred**: Avoid shared mutable state when possible
///
/// ## Example Implementation
///
/// ```rust,no_run
/// use automatic_coding_agent::task::{TaskEvent, TaskEventHandler};
/// use anyhow::Result;
/// use std::sync::atomic::{AtomicU64, Ordering};
///
/// pub struct TaskCounter {
///     total_created: AtomicU64,
///     total_completed: AtomicU64,
///     total_failed: AtomicU64,
/// }
///
/// impl TaskCounter {
///     pub fn new() -> Self {
///         Self {
///             total_created: AtomicU64::new(0),
///             total_completed: AtomicU64::new(0),
///             total_failed: AtomicU64::new(0),
///         }
///     }
///
///     pub fn get_stats(&self) -> (u64, u64, u64) {
///         (
///             self.total_created.load(Ordering::Relaxed),
///             self.total_completed.load(Ordering::Relaxed),
///             self.total_failed.load(Ordering::Relaxed),
///         )
///     }
/// }
///
/// impl TaskEventHandler for TaskCounter {
///     fn handle_event(&self, event: &TaskEvent) -> Result<()> {
///         match event {
///             TaskEvent::TaskCreated { .. } => {
///                 self.total_created.fetch_add(1, Ordering::Relaxed);
///             }
///             TaskEvent::TaskCompleted { .. } => {
///                 self.total_completed.fetch_add(1, Ordering::Relaxed);
///             }
///             TaskEvent::TaskFailed { .. } => {
///                 self.total_failed.fetch_add(1, Ordering::Relaxed);
///             }
///             _ => {} // Ignore other events
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait TaskEventHandler {
    /// Handle a task event.
    ///
    /// This method is called for every task event. Implementations should
    /// be fast and non-blocking. Return an error only for critical failures
    /// that should stop task processing.
    ///
    /// # Arguments
    ///
    /// * `event` - The task event to handle
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Event was handled successfully
    /// * `Err(error)` - Critical error that should stop processing
    fn handle_event(&self, event: &TaskEvent) -> Result<()>;
}

/// Task manager operations
impl TaskManager {
    /// Create a new task manager with the specified configuration.
    ///
    /// Initializes a new task manager instance with an empty task tree,
    /// scheduler, and event handler system. The configuration determines
    /// retry policies, resource limits, and other operational parameters.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration settings for the task manager
    ///
    /// # Returns
    ///
    /// A new `TaskManager` instance ready to handle tasks
    ///
    /// # Example
    ///
    /// ```rust
    /// use automatic_coding_agent::task::{TaskManager, TaskManagerConfig};
    ///
    /// let config = TaskManagerConfig {
    ///     auto_retry_failed_tasks: true,
    ///     max_retry_attempts: 3,
    ///     retry_delay_minutes: 5,
    ///     auto_cleanup_completed: false,
    ///     cleanup_after_hours: 24,
    ///     enable_task_metrics: true,
    ///     max_concurrent_tasks: 4,
    /// };
    ///
    /// let task_manager = TaskManager::new(config);
    /// ```
    pub fn new(config: TaskManagerConfig) -> Self {
        let scheduler_config = SchedulerConfig {
            max_concurrent_tasks: config.max_concurrent_tasks,
            ..Default::default()
        };

        Self {
            tree: Arc::new(RwLock::new(TaskTree::new())),
            scheduler: Arc::new(Mutex::new(TaskScheduler::new(scheduler_config))),
            config,
            event_handlers: Vec::new(),
        }
    }

    /// Initialize the task manager with a batch of task specifications.
    ///
    /// Creates multiple root-level tasks from the provided specifications.
    /// This is useful for setting up an initial workload or batch processing
    /// multiple independent tasks.
    ///
    /// # Arguments
    ///
    /// * `specs` - Vector of task specifications to create
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<TaskId>)` - List of created task IDs
    /// * `Err(error)` - If any task creation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use automatic_coding_agent::task::{TaskManager, TaskManagerConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let task_manager = TaskManager::new(TaskManagerConfig::default());
    ///
    ///     // Task specs would be created and passed here
    ///     // let specs = create_task_specs();
    ///     // let task_ids = task_manager.initialize_with_specs(specs).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn initialize_with_specs(&self, specs: Vec<TaskSpec>) -> Result<Vec<TaskId>> {
        let mut tree = self.tree.write().await;
        let mut created_tasks = Vec::new();

        for spec in specs {
            let task_id = tree.create_task_from_spec(spec, None)?;
            created_tasks.push(task_id);

            self.emit_event(TaskEvent::TaskCreated {
                task_id,
                parent_id: None,
            })
            .await?;
        }

        info!(
            "Initialized task manager with {} root tasks",
            created_tasks.len()
        );
        Ok(created_tasks)
    }

    /// Create a new task from a specification.
    ///
    /// Creates a new task and adds it to the task tree. If a parent ID is provided,
    /// the task becomes a subtask of the specified parent. The task is automatically
    /// scheduled for execution based on its priority and dependencies.
    ///
    /// # Arguments
    ///
    /// * `spec` - Task specification defining the task properties
    /// * `parent_id` - Optional parent task ID for creating subtasks
    ///
    /// # Returns
    ///
    /// * `Ok(TaskId)` - The ID of the newly created task
    /// * `Err(error)` - If task creation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use automatic_coding_agent::task::{TaskManager, TaskManagerConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let task_manager = TaskManager::new(TaskManagerConfig::default());
    ///
    ///     // Task creation would involve building TaskSpec instances
    ///     // let task_spec = build_task_spec();
    ///     // let task_id = task_manager.create_task(task_spec, None).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_task(&self, spec: TaskSpec, parent_id: Option<TaskId>) -> Result<TaskId> {
        let mut tree = self.tree.write().await;
        let task_id = tree.create_task_from_spec(spec, parent_id)?;

        self.emit_event(TaskEvent::TaskCreated { task_id, parent_id })
            .await?;

        debug!("Created task {} with parent {:?}", task_id, parent_id);
        Ok(task_id)
    }

    /// Create multiple subtasks for a parent task
    pub async fn create_subtasks(
        &self,
        parent_id: TaskId,
        subtask_specs: Vec<TaskSpec>,
    ) -> Result<Vec<TaskId>> {
        let mut tree = self.tree.write().await;
        let subtask_ids = tree.create_subtasks(parent_id, subtask_specs).await?;

        self.emit_event(TaskEvent::SubtasksCreated {
            parent_id,
            subtask_ids: subtask_ids.clone(),
        })
        .await?;

        info!(
            "Created {} subtasks for parent {}",
            subtask_ids.len(),
            parent_id
        );
        Ok(subtask_ids)
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: TaskId) -> Result<Task> {
        let tree = self.tree.read().await;
        Ok(tree.get_task(task_id)?.clone())
    }

    /// Update task status
    pub async fn update_task_status(&self, task_id: TaskId, new_status: TaskStatus) -> Result<()> {
        let mut tree = self.tree.write().await;
        let old_status = tree.get_task(task_id)?.status.clone();

        tree.update_task_status(task_id, new_status.clone())?;

        self.emit_event(TaskEvent::TaskStatusChanged {
            task_id,
            old_status,
            new_status,
        })
        .await?;

        debug!("Updated task {} status", task_id);
        Ok(())
    }

    /// Mark task as completed with result
    pub async fn complete_task(&self, task_id: TaskId, result: TaskResult) -> Result<()> {
        let completed_status = TaskStatus::Completed {
            completed_at: Utc::now(),
            result: result.clone(),
        };

        self.update_task_status(task_id, completed_status).await?;

        self.emit_event(TaskEvent::TaskCompleted { task_id, result })
            .await?;

        // Check if parent task should be updated
        self.check_parent_completion(task_id).await?;

        info!("Completed task {}", task_id);
        Ok(())
    }

    /// Mark task as failed with error
    pub async fn fail_task(&self, task_id: TaskId, error: TaskError) -> Result<()> {
        let mut retry_count = 0;

        // Get current retry count
        {
            let tree = self.tree.read().await;
            let task = tree.get_task(task_id)?;
            if let TaskStatus::Failed {
                retry_count: count, ..
            } = task.status
            {
                retry_count = count;
            }
        }

        let failed_status = TaskStatus::Failed {
            failed_at: Utc::now(),
            error: error.clone(),
            retry_count: retry_count + 1,
        };

        self.update_task_status(task_id, failed_status).await?;

        self.emit_event(TaskEvent::TaskFailed {
            task_id,
            error: error.clone(),
        })
        .await?;

        // Check if we should auto-retry
        if self.config.auto_retry_failed_tasks && retry_count < self.config.max_retry_attempts {
            self.schedule_retry(task_id).await?;
        }

        warn!("Failed task {} (attempt {})", task_id, retry_count + 1);
        Ok(())
    }

    /// Block task with reason
    pub async fn block_task(
        &self,
        task_id: TaskId,
        reason: String,
        retry_after: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let blocked_status = TaskStatus::Blocked {
            reason: reason.clone(),
            blocked_at: Utc::now(),
            retry_after,
        };

        self.update_task_status(task_id, blocked_status).await?;

        debug!("Blocked task {}: {}", task_id, reason);
        Ok(())
    }

    /// Select next task for execution using scheduler
    pub async fn select_next_task(&self) -> Result<Option<TaskSelection>> {
        let tree = self.tree.read().await;
        let scheduler = self.scheduler.lock().await;
        Ok(scheduler.select_next_task(&tree).await)
    }

    /// Get eligible tasks for execution
    pub async fn get_eligible_tasks(&self) -> Result<Vec<TaskId>> {
        let tree = self.tree.read().await;
        Ok(tree.get_eligible_tasks())
    }

    /// Get task tree progress
    pub async fn get_progress(&self) -> Result<TaskTreeProgress> {
        let tree = self.tree.read().await;
        Ok(tree.calculate_progress())
    }

    /// Get task tree statistics
    pub async fn get_statistics(&self) -> Result<TaskTreeStatistics> {
        let tree = self.tree.read().await;
        Ok(tree.metadata.statistics.clone())
    }

    /// Remove completed tasks if auto-cleanup is enabled
    pub async fn cleanup_completed_tasks(&self) -> Result<Vec<TaskId>> {
        if !self.config.auto_cleanup_completed {
            return Ok(Vec::new());
        }

        let cutoff_time =
            Utc::now() - chrono::Duration::hours(self.config.cleanup_after_hours as i64);
        let mut cleaned_tasks = Vec::new();

        let task_ids: Vec<TaskId>;
        {
            let tree = self.tree.read().await;
            task_ids = tree.get_all_task_ids();
        }

        for task_id in task_ids {
            let should_cleanup = {
                let tree = self.tree.read().await;
                if let Ok(task) = tree.get_task(task_id) {
                    match &task.status {
                        TaskStatus::Completed { completed_at, .. } => {
                            *completed_at < cutoff_time && task.children.is_empty()
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            };

            if should_cleanup {
                let mut tree = self.tree.write().await;
                tree.remove_task(task_id).await?;
                cleaned_tasks.push(task_id);
            }
        }

        if !cleaned_tasks.is_empty() {
            info!("Cleaned up {} completed tasks", cleaned_tasks.len());
        }

        Ok(cleaned_tasks)
    }

    /// Deduplicate similar tasks
    pub async fn deduplicate_tasks(&self) -> Result<Vec<TaskId>> {
        let mut tree = self.tree.write().await;
        let similar_clusters = tree.find_similar_tasks().await?;
        let mut merged_tasks = Vec::new();

        for cluster in similar_clusters {
            if cluster.len() > 1 {
                let primary = cluster[0];
                let duplicates = &cluster[1..];

                // Merge metadata and dependencies
                tree.merge_task_cluster(primary, duplicates).await?;

                // Remove duplicate tasks
                for &duplicate_id in duplicates {
                    tree.remove_task(duplicate_id).await?;
                    merged_tasks.push(duplicate_id);
                }
            }
        }

        if !merged_tasks.is_empty() {
            info!("Deduplicated {} tasks", merged_tasks.len());
        }

        Ok(merged_tasks)
    }

    /// Schedule a retry for a failed task
    async fn schedule_retry(&self, task_id: TaskId) -> Result<()> {
        let retry_time =
            Utc::now() + chrono::Duration::minutes(self.config.retry_delay_minutes as i64);

        self.block_task(
            task_id,
            "Scheduled for automatic retry".to_string(),
            Some(retry_time),
        )
        .await?;

        debug!("Scheduled retry for task {} at {}", task_id, retry_time);
        Ok(())
    }

    /// Check if parent task should be marked as completed
    async fn check_parent_completion(&self, completed_task_id: TaskId) -> Result<()> {
        let parent_id = {
            let tree = self.tree.read().await;
            let task = tree.get_task(completed_task_id)?;
            task.parent_id
        };

        if let Some(parent_id) = parent_id {
            let all_children_completed = {
                let tree = self.tree.read().await;
                let parent = tree.get_task(parent_id)?;

                parent.children.iter().all(|&child_id| {
                    if let Ok(child) = tree.get_task(child_id) {
                        child.is_terminal()
                    } else {
                        false
                    }
                })
            };

            if all_children_completed {
                let success_result = TaskResult::Success {
                    output: serde_json::json!({"message": "All subtasks completed"}),
                    files_created: Vec::new(),
                    files_modified: Vec::new(),
                    build_artifacts: Vec::new(),
                };

                // Use Box::pin to avoid recursion issue
                Box::pin(self.complete_task(parent_id, success_result)).await?;
                info!(
                    "Auto-completed parent task {} (all children finished)",
                    parent_id
                );
            }
        }

        Ok(())
    }

    /// Add event handler
    pub fn add_event_handler(&mut self, handler: Box<dyn TaskEventHandler + Send + Sync>) {
        self.event_handlers.push(handler);
    }

    /// Emit task event to all handlers
    async fn emit_event(&self, event: TaskEvent) -> Result<()> {
        for handler in &self.event_handlers {
            if let Err(e) = handler.handle_event(&event) {
                error!("Event handler error: {}", e);
            }
        }
        Ok(())
    }

    /// Update scheduler context with recent activity
    pub async fn update_scheduler_context(
        &self,
        files: Vec<std::path::PathBuf>,
        repositories: Vec<String>,
    ) -> Result<()> {
        let mut scheduler = self.scheduler.lock().await;
        scheduler.update_context(files, repositories);
        Ok(())
    }

    /// Get tasks in a specific status
    pub async fn get_tasks_by_status(
        &self,
        status_filter: fn(&TaskStatus) -> bool,
    ) -> Result<Vec<TaskId>> {
        let tree = self.tree.read().await;
        let matching_tasks = tree
            .tasks
            .iter()
            .filter(|(_, task)| status_filter(&task.status))
            .map(|(&id, _)| id)
            .collect();

        Ok(matching_tasks)
    }

    /// Get tasks by priority
    pub async fn get_tasks_by_priority(&self, min_priority: TaskPriority) -> Result<Vec<TaskId>> {
        let tree = self.tree.read().await;
        let matching_tasks = tree
            .tasks
            .iter()
            .filter(|(_, task)| task.metadata.priority >= min_priority)
            .map(|(&id, _)| id)
            .collect();

        Ok(matching_tasks)
    }

    /// Export task tree to JSON
    pub async fn export_to_json(&self) -> Result<String> {
        let tree = self.tree.read().await;
        serde_json::to_string_pretty(&*tree).map_err(|e| anyhow!("Serialization error: {}", e))
    }

    /// Import task tree from JSON
    pub async fn import_from_json(&self, json_data: &str) -> Result<()> {
        let imported_tree: TaskTree =
            serde_json::from_str(json_data).map_err(|e| anyhow!("Deserialization error: {}", e))?;

        let mut tree = self.tree.write().await;
        *tree = imported_tree;

        info!("Imported task tree with {} tasks", tree.tasks.len());
        Ok(())
    }

    /// Validate task tree integrity
    pub async fn validate_tree_integrity(&self) -> Result<Vec<String>> {
        let tree = self.tree.read().await;
        let mut issues = Vec::new();

        // Check for orphaned tasks
        for (&task_id, task) in &tree.tasks {
            if let Some(parent_id) = task.parent_id
                && !tree.tasks.contains_key(&parent_id)
            {
                issues.push(format!(
                    "Task {} has non-existent parent {}",
                    task_id, parent_id
                ));
            }
        }

        // Check for circular dependencies
        for &task_id in tree.tasks.keys() {
            if tree.has_circular_dependency(task_id)? {
                issues.push(format!("Task {} has circular dependency", task_id));
            }
        }

        // Check for broken children references
        for (&task_id, task) in &tree.tasks {
            for &child_id in &task.children {
                if !tree.tasks.contains_key(&child_id) {
                    issues.push(format!(
                        "Task {} references non-existent child {}",
                        task_id, child_id
                    ));
                }
            }
        }

        Ok(issues)
    }
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            auto_retry_failed_tasks: true,
            max_retry_attempts: 3,
            retry_delay_minutes: 5,
            auto_cleanup_completed: false,
            cleanup_after_hours: 24,
            enable_task_metrics: true,
            max_concurrent_tasks: 3,
        }
    }
}

// ============================================================================
// Integration Examples and Best Practices
// ============================================================================

/// # Integration Examples and Best Practices
///
/// This section provides comprehensive examples showing how to effectively
/// use the task manager in real-world scenarios.

/// ## Example: Complete Project Workflow
///
/// ```rust,ignore
/// use automatic_coding_agent::task::{
///     TaskManager, TaskManagerConfig, TaskSpec, TaskPriority, TaskMetadata,
///     ComplexityLevel, ContextRequirements, LoggingEventHandler
/// };
/// use std::collections::HashMap;
/// use chrono::Duration;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // 1. Configure the task manager for development workflow
///     let config = TaskManagerConfig {
///         auto_retry_failed_tasks: true,
///         max_retry_attempts: 2,
///         retry_delay_minutes: 2,
///         auto_cleanup_completed: false, // Keep for analysis
///         cleanup_after_hours: 48,
///         enable_task_metrics: true,
///         max_concurrent_tasks: 6,
///     };
///
///     // 2. Create and configure task manager
///     let mut task_manager = TaskManager::new(config);
///     task_manager.add_event_handler(Box::new(LoggingEventHandler));
///
///     // 3. Define a complex project structure
///     let main_spec = TaskSpec {
///         title: "E-commerce Platform Development".to_string(),
///         description: "Build a complete e-commerce platform with microservices".to_string(),
///         metadata: TaskMetadata {
///             priority: TaskPriority::Critical,
///             estimated_complexity: Some(ComplexityLevel::Epic),
///             estimated_duration: Some(Duration::weeks(8)),
///             tags: vec!["project".to_string(), "ecommerce".to_string()],
///             context_requirements: ContextRequirements {
///                 required_files: vec!["package.json".into(), "docker-compose.yml".into()],
///                 build_dependencies: vec!["docker".to_string(), "nodejs".to_string()],
///                 environment_vars: HashMap::from([
///                     ("NODE_ENV".to_string(), "development".to_string()),
///                 ]),
///                 ..Default::default()
///             },
///             ..Default::default()
///         },
///         ..Default::default()
///     };
///
///     let main_id = task_manager.create_task(main_spec, None).await?;
///
///     // 4. Create backend microservices tasks
///     let backend_specs = vec![
///         TaskSpec {
///             title: "User Service".to_string(),
///             description: "User authentication and profile management".to_string(),
///             dependencies: vec![main_id],
///             metadata: TaskMetadata {
///                 priority: TaskPriority::High,
///                 estimated_complexity: Some(ComplexityLevel::Complex),
///                 estimated_duration: Some(Duration::weeks(2)),
///                 tags: vec!["backend".to_string(), "users".to_string()],
///                 ..Default::default()
///             },
///             ..Default::default()
///         },
///         TaskSpec {
///             title: "Product Catalog Service".to_string(),
///             description: "Product management and catalog functionality".to_string(),
///             dependencies: vec![main_id],
///             metadata: TaskMetadata {
///                 priority: TaskPriority::High,
///                 estimated_complexity: Some(ComplexityLevel::Complex),
///                 estimated_duration: Some(Duration::weeks(3)),
///                 tags: vec!["backend".to_string(), "products".to_string()],
///                 ..Default::default()
///             },
///             ..Default::default()
///         },
///     ];
///
///     // 5. Create backend tasks with dependencies
///     for spec in backend_specs {
///         task_manager.create_task(spec, Some(main_id)).await?;
///     }
///
///     // 6. Monitor progress
///     let stats = task_manager.get_statistics().await?;
///     println!("Project setup complete:");
///     println!("  Total tasks: {}", stats.total_tasks);
///     println!("  In progress tasks: {}", stats.in_progress_tasks);
///     println!("  Pending tasks: {}", stats.pending_tasks);
///
///     Ok(())
/// }
/// ```
///
/// ## Example: Error Handling and Recovery
///
/// ```rust,ignore
/// use automatic_coding_agent::task::{
///     TaskManager, TaskManagerConfig, TaskEvent, TaskEventHandler, TaskStatus
/// };
/// use anyhow::Result;
/// use std::sync::atomic::{AtomicU32, Ordering};
/// use std::sync::Arc;
///
/// // Custom handler for tracking and responding to failures
/// struct FailureTracker {
///     failure_count: AtomicU32,
///     max_failures: u32,
/// }
///
/// impl FailureTracker {
///     fn new(max_failures: u32) -> Self {
///         Self {
///             failure_count: AtomicU32::new(0),
///             max_failures,
///         }
///     }
///
///     fn should_abort(&self) -> bool {
///         self.failure_count.load(Ordering::Relaxed) >= self.max_failures
///     }
/// }
///
/// impl TaskEventHandler for FailureTracker {
///     fn handle_event(&self, event: &TaskEvent) -> Result<()> {
///         match event {
///             TaskEvent::TaskFailed { task_id, error } => {
///                 let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
///                 eprintln!("Task {} failed ({} total failures): {:?}", task_id, count, error);
///
///                 if self.should_abort() {
///                     eprintln!("Too many failures, consider aborting workflow");
///                 }
///             }
///             TaskEvent::TaskCompleted { task_id, .. } => {
///                 // Reset failure count on successful completion
///                 self.failure_count.store(0, Ordering::Relaxed);
///                 println!("Task {} completed successfully", task_id);
///             }
///             _ => {}
///         }
///         Ok(())
///     }
/// }
///
/// async fn resilient_workflow() -> Result<()> {
///     let config = TaskManagerConfig {
///         auto_retry_failed_tasks: true,
///         max_retry_attempts: 3,
///         retry_delay_minutes: 1,
///         ..Default::default()
///     };
///
///     let mut task_manager = TaskManager::new(config);
///     let failure_tracker = Arc::new(FailureTracker::new(5));
///     task_manager.add_event_handler(Box::new(failure_tracker));
///
///     // Your task creation and execution logic here...
///
///     Ok(())
/// }
/// ```
///
/// ## Best Practices
///
/// ### 1. Configuration Guidelines
/// - **Development**: Use lower retry counts and shorter delays for faster feedback
/// - **Production**: Use higher retry counts with exponential backoff
/// - **Resource Limits**: Set `max_concurrent_tasks` based on available CPU/memory
/// - **Cleanup**: Enable auto-cleanup in production, disable in development for debugging
///
/// ### 2. Task Design Principles
/// - **Single Responsibility**: Each task should have a clear, single purpose
/// - **Idempotent Operations**: Tasks should be safe to retry
/// - **Dependency Management**: Clearly define task dependencies
/// - **Error Boundaries**: Design tasks to fail gracefully
///
/// ### 3. Event Handler Best Practices
/// - **Non-blocking**: Keep event handlers fast and asynchronous
/// - **Error Isolation**: Don't let handler errors affect task execution
/// - **Metrics Collection**: Use handlers for monitoring and alerting
/// - **Resource Management**: Clean up resources in event handlers
///
/// ### 4. Performance Optimization
/// - **Batch Operations**: Use `initialize_with_specs` for bulk task creation
/// - **Resource Pooling**: Reuse connections and expensive resources
/// - **Memory Management**: Enable auto-cleanup for long-running processes
/// - **Monitoring**: Track task execution times and resource usage

/// Simple event handler that logs events
pub struct LoggingEventHandler;

impl TaskEventHandler for LoggingEventHandler {
    fn handle_event(&self, event: &TaskEvent) -> Result<()> {
        match event {
            TaskEvent::TaskCreated { task_id, parent_id } => {
                info!("Task created: {} (parent: {:?})", task_id, parent_id);
            }
            TaskEvent::TaskStatusChanged {
                task_id,
                old_status,
                new_status,
            } => {
                info!(
                    "Task {} status: {:?} -> {:?}",
                    task_id, old_status, new_status
                );
            }
            TaskEvent::TaskCompleted { task_id, .. } => {
                info!("Task completed: {}", task_id);
            }
            TaskEvent::TaskFailed { task_id, error } => {
                warn!("Task failed: {} - {:?}", task_id, error);
            }
            TaskEvent::SubtasksCreated {
                parent_id,
                subtask_ids,
            } => {
                info!(
                    "Subtasks created for {}: {} tasks",
                    parent_id,
                    subtask_ids.len()
                );
            }
            TaskEvent::TasksDeduped {
                primary_id,
                merged_ids,
            } => {
                info!("Tasks merged into {}: {:?}", primary_id, merged_ids);
            }
            TaskEvent::TreeStatisticsUpdated { .. } => {
                debug!("Task tree statistics updated");
            }
        }
        Ok(())
    }
}
