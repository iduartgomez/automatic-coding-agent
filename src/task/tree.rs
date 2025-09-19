use crate::task::types::*;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// Task tree for managing hierarchical task relationships
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskTree {
    /// All tasks indexed by ID
    pub tasks: HashMap<TaskId, Task>,
    /// Root task IDs (tasks with no parent)
    pub roots: Vec<TaskId>,
    /// Task tree metadata
    pub metadata: TaskTreeMetadata,
}

/// Metadata for the task tree
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskTreeMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
    pub total_tasks_created: u32,
    pub statistics: TaskTreeStatistics,
}

/// Task tree statistics for monitoring
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskTreeStatistics {
    pub total_tasks: u32,
    pub pending_tasks: u32,
    pub in_progress_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub blocked_tasks: u32,
    pub skipped_tasks: u32,
    pub average_completion_time: Option<Duration>,
    pub success_rate: f64,
}

/// Progress information for the entire task tree
#[derive(Debug, Clone, Serialize)]
pub struct TaskTreeProgress {
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub in_progress_tasks: u32,
    pub blocked_tasks: u32,
    pub failed_tasks: u32,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub current_throughput: f64, // tasks per hour
    pub completion_percentage: f64,
}

impl TaskTree {
    /// Create a new empty task tree
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            tasks: HashMap::new(),
            roots: Vec::new(),
            metadata: TaskTreeMetadata {
                created_at: now,
                updated_at: now,
                version: 1,
                total_tasks_created: 0,
                statistics: TaskTreeStatistics::default(),
            },
        }
    }

    /// Create task tree from a specification
    pub fn from_specification(specs: Vec<TaskSpec>) -> Result<Self> {
        let mut tree = Self::new();

        for spec in specs {
            tree.create_task_from_spec(spec, None)?;
        }

        tree.rebuild_statistics();
        Ok(tree)
    }

    /// Add a new task to the tree
    pub fn add_task(&mut self, task: Task) -> Result<TaskId> {
        let task_id = task.id;

        // Validate parent exists if specified
        if let Some(parent_id) = task.parent_id {
            if !self.tasks.contains_key(&parent_id) {
                return Err(anyhow!("Parent task {} does not exist", parent_id));
            }

            // Add this task to parent's children
            if let Some(parent) = self.tasks.get_mut(&parent_id) {
                if !parent.children.contains(&task_id) {
                    parent.children.push(task_id);
                }
            }
        } else {
            // This is a root task
            if !self.roots.contains(&task_id) {
                self.roots.push(task_id);
            }
        }

        // Validate dependencies exist
        for &dep_id in &task.dependencies {
            if !self.tasks.contains_key(&dep_id) {
                warn!(
                    "Task {} has dependency {} that doesn't exist yet",
                    task_id, dep_id
                );
            }
        }

        self.tasks.insert(task_id, task);
        self.metadata.total_tasks_created += 1;
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;

        debug!("Added task {} to tree", task_id);
        Ok(task_id)
    }

    /// Create a task from a specification
    pub fn create_task_from_spec(
        &mut self,
        spec: TaskSpec,
        parent_id: Option<TaskId>,
    ) -> Result<TaskId> {
        let task = Task::new(spec, parent_id);
        self.add_task(task)
    }

    /// Get a task by ID
    pub fn get_task(&self, task_id: TaskId) -> Result<&Task> {
        self.tasks
            .get(&task_id)
            .ok_or_else(|| anyhow!("Task {} not found", task_id))
    }

    /// Get a mutable reference to a task by ID
    pub fn get_task_mut(&mut self, task_id: TaskId) -> Result<&mut Task> {
        self.tasks
            .get_mut(&task_id)
            .ok_or_else(|| anyhow!("Task {} not found", task_id))
    }

    /// Get all task IDs
    pub fn get_all_task_ids(&self) -> Vec<TaskId> {
        self.tasks.keys().cloned().collect()
    }

    /// Get all root task IDs
    pub fn get_root_task_ids(&self) -> &[TaskId] {
        &self.roots
    }

    /// Get children of a task
    pub fn get_children(&self, task_id: TaskId) -> Result<Vec<&Task>> {
        let task = self.get_task(task_id)?;
        let mut children = Vec::new();

        for &child_id in &task.children {
            if let Ok(child) = self.get_task(child_id) {
                children.push(child);
            }
        }

        Ok(children)
    }

    /// Get parent of a task
    pub fn get_parent(&self, task_id: TaskId) -> Result<Option<&Task>> {
        let task = self.get_task(task_id)?;
        if let Some(parent_id) = task.parent_id {
            Ok(Some(self.get_task(parent_id)?))
        } else {
            Ok(None)
        }
    }

    /// Create subtasks dynamically during parent task execution
    pub async fn create_subtasks(
        &mut self,
        parent_id: TaskId,
        subtask_specs: Vec<TaskSpec>,
    ) -> Result<Vec<TaskId>> {
        let parent = self.get_task_mut(parent_id)?;

        // Update parent status if it's not already in progress
        if matches!(parent.status, TaskStatus::Pending) {
            parent.status = TaskStatus::InProgress {
                started_at: Utc::now(),
                estimated_completion: None,
            };
            parent.updated_at = Utc::now();
        }

        let mut created_tasks = Vec::new();

        for spec in subtask_specs {
            let task_id = self.create_task_from_spec(spec, Some(parent_id))?;
            created_tasks.push(task_id);
        }

        self.recalculate_dependencies().await?;
        self.rebuild_statistics();

        info!(
            "Created {} subtasks for parent {}",
            created_tasks.len(),
            parent_id
        );
        Ok(created_tasks)
    }

    /// Remove a task and update relationships
    pub async fn remove_task(&mut self, task_id: TaskId) -> Result<()> {
        let task = self.get_task(task_id)?.clone();

        // Remove from parent's children
        if let Some(parent_id) = task.parent_id {
            if let Ok(parent) = self.get_task_mut(parent_id) {
                parent.children.retain(|&id| id != task_id);
            }
        } else {
            // Remove from roots
            self.roots.retain(|&id| id != task_id);
        }

        // Reassign children to parent or make them roots
        for &child_id in &task.children {
            if let Ok(child) = self.get_task_mut(child_id) {
                child.parent_id = task.parent_id;
                if task.parent_id.is_none() {
                    self.roots.push(child_id);
                } else if let Ok(grandparent) = self.get_task_mut(task.parent_id.unwrap()) {
                    grandparent.children.push(child_id);
                }
            }
        }

        // Remove task dependencies from other tasks
        for other_task in self.tasks.values_mut() {
            other_task.dependencies.retain(|&id| id != task_id);
        }

        self.tasks.remove(&task_id);
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;
        self.rebuild_statistics();

        info!("Removed task {} from tree", task_id);
        Ok(())
    }

    /// Update task status
    pub fn update_task_status(&mut self, task_id: TaskId, status: TaskStatus) -> Result<()> {
        let task = self.get_task_mut(task_id)?;
        task.update_status(status);
        self.metadata.updated_at = Utc::now();
        self.rebuild_statistics();
        Ok(())
    }

    /// Check if all dependencies of a task are satisfied
    pub fn are_dependencies_satisfied(&self, task_id: TaskId) -> Result<bool> {
        let task = self.get_task(task_id)?;

        for &dep_id in &task.dependencies {
            let dep_task = self.get_task(dep_id)?;
            if !matches!(dep_task.status, TaskStatus::Completed { .. }) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get tasks that are eligible for execution
    pub fn get_eligible_tasks(&self) -> Vec<TaskId> {
        let mut eligible = Vec::new();

        for (&task_id, task) in &self.tasks {
            if task.is_runnable() {
                if let Ok(deps_satisfied) = self.are_dependencies_satisfied(task_id) {
                    if deps_satisfied {
                        eligible.push(task_id);
                    }
                }
            }
        }

        eligible
    }

    /// Find tasks similar to each other for deduplication
    pub async fn find_similar_tasks(&self) -> Result<Vec<Vec<TaskId>>> {
        let mut clusters = Vec::new();
        let mut processed = HashSet::new();

        for (&task_id, task) in &self.tasks {
            if processed.contains(&task_id) {
                continue;
            }

            let mut cluster = vec![task_id];
            processed.insert(task_id);

            // Find similar tasks
            for (&other_id, other_task) in &self.tasks {
                if other_id != task_id
                    && !processed.contains(&other_id)
                    && self.tasks_are_similar(task, other_task)
                {
                    cluster.push(other_id);
                    processed.insert(other_id);
                }
            }

            if cluster.len() > 1 {
                clusters.push(cluster);
            }
        }

        Ok(clusters)
    }

    /// Check if two tasks are similar enough to be merged
    fn tasks_are_similar(&self, task1: &Task, task2: &Task) -> bool {
        // Simple similarity check based on title and description similarity
        let title_similarity = self.string_similarity(&task1.title, &task2.title);
        let desc_similarity = self.string_similarity(&task1.description, &task2.description);

        title_similarity > 0.8 || desc_similarity > 0.7
    }

    /// Calculate string similarity (simple implementation)
    fn string_similarity(&self, s1: &str, s2: &str) -> f64 {
        let s1_words: HashSet<&str> = s1.split_whitespace().collect();
        let s2_words: HashSet<&str> = s2.split_whitespace().collect();

        let intersection = s1_words.intersection(&s2_words).count();
        let union = s1_words.union(&s2_words).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Merge a cluster of duplicate tasks
    pub async fn merge_task_cluster(
        &mut self,
        primary_id: TaskId,
        duplicate_ids: &[TaskId],
    ) -> Result<()> {
        let _primary = self.get_task(primary_id)?.clone();

        for &duplicate_id in duplicate_ids {
            let duplicate = self.get_task(duplicate_id)?.clone();

            // Merge dependencies
            let primary_task = self.get_task_mut(primary_id)?;
            for dep in duplicate.dependencies {
                if !primary_task.dependencies.contains(&dep) {
                    primary_task.dependencies.push(dep);
                }
            }

            // Merge tags
            for tag in duplicate.metadata.tags {
                if !primary_task.metadata.tags.contains(&tag) {
                    primary_task.metadata.tags.push(tag);
                }
            }

            // Merge file references
            for file_ref in duplicate.metadata.file_refs {
                if !primary_task
                    .metadata
                    .file_refs
                    .iter()
                    .any(|f| f.path == file_ref.path)
                {
                    primary_task.metadata.file_refs.push(file_ref);
                }
            }
        }

        Ok(())
    }

    /// Recalculate task dependencies after changes
    pub async fn recalculate_dependencies(&mut self) -> Result<()> {
        // This is a placeholder for dependency graph analysis
        // In a full implementation, this would detect circular dependencies,
        // optimize dependency chains, and validate the dependency graph

        // Check for circular dependencies
        for &task_id in self.tasks.keys() {
            if self.has_circular_dependency(task_id)? {
                warn!("Circular dependency detected for task {}", task_id);
            }
        }

        Ok(())
    }

    /// Check if a task has circular dependencies
    pub fn has_circular_dependency(&self, task_id: TaskId) -> Result<bool> {
        let mut visited = HashSet::new();
        let mut path = HashSet::new();
        self.has_circular_dependency_helper(task_id, &mut visited, &mut path)
    }

    /// Helper function for circular dependency detection
    fn has_circular_dependency_helper(
        &self,
        task_id: TaskId,
        visited: &mut HashSet<TaskId>,
        path: &mut HashSet<TaskId>,
    ) -> Result<bool> {
        if path.contains(&task_id) {
            return Ok(true); // Circular dependency found
        }

        if visited.contains(&task_id) {
            return Ok(false); // Already processed
        }

        visited.insert(task_id);
        path.insert(task_id);

        let task = self.get_task(task_id)?;
        for &dep_id in &task.dependencies {
            if self.has_circular_dependency_helper(dep_id, visited, path)? {
                return Ok(true);
            }
        }

        path.remove(&task_id);
        Ok(false)
    }

    /// Calculate progress for the entire task tree
    pub fn calculate_progress(&self) -> TaskTreeProgress {
        let stats = &self.metadata.statistics;

        let total = stats.total_tasks as f64;
        let completed = stats.completed_tasks as f64;

        TaskTreeProgress {
            total_tasks: stats.total_tasks,
            completed_tasks: stats.completed_tasks,
            in_progress_tasks: stats.in_progress_tasks,
            blocked_tasks: stats.blocked_tasks,
            failed_tasks: stats.failed_tasks,
            estimated_completion: self.estimate_total_completion(),
            current_throughput: self.calculate_throughput(),
            completion_percentage: if total > 0.0 {
                (completed / total) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Estimate when all tasks will be completed
    fn estimate_total_completion(&self) -> Option<DateTime<Utc>> {
        let remaining_tasks = self.metadata.statistics.total_tasks
            - self.metadata.statistics.completed_tasks
            - self.metadata.statistics.failed_tasks
            - self.metadata.statistics.skipped_tasks;

        if remaining_tasks == 0 {
            return Some(Utc::now());
        }

        let throughput = self.calculate_throughput();
        if throughput > 0.0 {
            let hours_remaining = remaining_tasks as f64 / throughput;
            Some(Utc::now() + Duration::milliseconds((hours_remaining * 3600.0 * 1000.0) as i64))
        } else {
            None
        }
    }

    /// Calculate current task completion throughput (tasks per hour)
    fn calculate_throughput(&self) -> f64 {
        // Simple implementation - in practice would use sliding window
        let elapsed_hours = self
            .metadata
            .created_at
            .signed_duration_since(Utc::now())
            .num_hours()
            .abs() as f64;
        if elapsed_hours > 0.0 {
            self.metadata.statistics.completed_tasks as f64 / elapsed_hours
        } else {
            0.0
        }
    }

    /// Rebuild statistics from current task states
    pub fn rebuild_statistics(&mut self) {
        let mut stats = TaskTreeStatistics {
            total_tasks: self.tasks.len() as u32,
            ..Default::default()
        };

        let mut completion_times = Vec::new();
        let mut successful_tasks = 0;

        for task in self.tasks.values() {
            match &task.status {
                TaskStatus::Pending => stats.pending_tasks += 1,
                TaskStatus::InProgress { .. } => stats.in_progress_tasks += 1,
                TaskStatus::Completed { completed_at, .. } => {
                    stats.completed_tasks += 1;
                    successful_tasks += 1;
                    let duration = completed_at.signed_duration_since(task.created_at);
                    completion_times.push(duration);
                }
                TaskStatus::Failed { .. } => stats.failed_tasks += 1,
                TaskStatus::Blocked { .. } => stats.blocked_tasks += 1,
                TaskStatus::Skipped { .. } => stats.skipped_tasks += 1,
            }
        }

        // Calculate average completion time
        if !completion_times.is_empty() {
            let total_time: Duration = completion_times.iter().sum();
            stats.average_completion_time = Some(total_time / completion_times.len() as i32);
        }

        // Calculate success rate
        let finished_tasks = stats.completed_tasks + stats.failed_tasks + stats.skipped_tasks;
        if finished_tasks > 0 {
            stats.success_rate = successful_tasks as f64 / finished_tasks as f64;
        }

        self.metadata.statistics = stats;
        self.metadata.updated_at = Utc::now();
    }
}

impl Default for TaskTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TaskTreeStatistics {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            pending_tasks: 0,
            in_progress_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            blocked_tasks: 0,
            skipped_tasks: 0,
            average_completion_time: None,
            success_rate: 0.0,
        }
    }
}

impl Task {
    /// Calculate effective context by merging parent context
    pub fn effective_context(&self, tree: &TaskTree) -> ContextRequirements {
        let mut context = self.metadata.context_requirements.clone();

        if let Some(parent_id) = self.parent_id {
            if let Ok(parent) = tree.get_task(parent_id) {
                let parent_context = parent.effective_context(tree);
                context.merge_with(&parent_context);
            }
        }

        context
    }
}
