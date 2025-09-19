use crate::task::scheduler::*;
use crate::task::tree::*;
use crate::task::types::*;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Central task management system
pub struct TaskManager {
    tree: Arc<RwLock<TaskTree>>,
    scheduler: Arc<Mutex<TaskScheduler>>,
    config: TaskManagerConfig,
    event_handlers: Vec<Box<dyn TaskEventHandler + Send + Sync>>,
}

/// Configuration for task manager
#[derive(Clone, Debug)]
pub struct TaskManagerConfig {
    pub auto_retry_failed_tasks: bool,
    pub max_retry_attempts: u32,
    pub retry_delay_minutes: u32,
    pub auto_cleanup_completed: bool,
    pub cleanup_after_hours: u32,
    pub enable_task_metrics: bool,
    pub max_concurrent_tasks: u32,
}

/// Events that can occur during task management
#[derive(Debug, Clone)]
pub enum TaskEvent {
    TaskCreated {
        task_id: TaskId,
        parent_id: Option<TaskId>,
    },
    TaskStatusChanged {
        task_id: TaskId,
        old_status: TaskStatus,
        new_status: TaskStatus,
    },
    TaskCompleted {
        task_id: TaskId,
        result: TaskResult,
    },
    TaskFailed {
        task_id: TaskId,
        error: TaskError,
    },
    SubtasksCreated {
        parent_id: TaskId,
        subtask_ids: Vec<TaskId>,
    },
    TasksDeduped {
        primary_id: TaskId,
        merged_ids: Vec<TaskId>,
    },
    TreeStatisticsUpdated {
        statistics: TaskTreeStatistics,
    },
}

/// Handler for task events
pub trait TaskEventHandler {
    fn handle_event(&self, event: &TaskEvent) -> Result<()>;
}

/// Task manager operations
impl TaskManager {
    /// Create a new task manager
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

    /// Initialize task manager with task specifications
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

    /// Create a new task
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
            if let Some(parent_id) = task.parent_id {
                if !tree.tasks.contains_key(&parent_id) {
                    issues.push(format!(
                        "Task {} has non-existent parent {}",
                        task_id, parent_id
                    ));
                }
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
