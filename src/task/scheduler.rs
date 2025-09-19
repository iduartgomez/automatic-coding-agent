use crate::task::tree::*;
use crate::task::types::*;
use chrono::{DateTime, Duration, Utc};
use rand::seq::SliceRandom;
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Task scheduler with intelligent prioritization
pub struct TaskScheduler {
    scoring_weights: ScoringWeights,
    resource_monitor: ResourceMonitor,
    context_cache: ContextCache,
    config: SchedulerConfig,
}

/// Weights for task scoring algorithm
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub priority_weight: f64,
    pub dependency_weight: f64,
    pub context_similarity_weight: f64,
    pub resource_availability_weight: f64,
    pub failure_penalty_weight: f64,
    pub age_bonus_weight: f64,
    pub complexity_weight: f64,
}

/// Resource monitoring for task execution
#[derive(Clone, Debug)]
pub struct ResourceMonitor {
    pub max_concurrent_tasks: u32,
    pub memory_limit_mb: u64,
    pub cpu_limit_percent: f64,
    pub current_usage: ResourceUsage,
}

/// Context cache for optimization
#[derive(Clone, Debug)]
pub struct ContextCache {
    recent_files: Vec<std::path::PathBuf>,
    active_repositories: Vec<String>,
    #[allow(dead_code)]
    claude_context_window: Vec<String>,
    last_updated: DateTime<Utc>,
}

/// Scheduler configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_concurrent_tasks: u32,
    pub selection_randomization: f64, // 0.0 = pure scoring, 1.0 = random
    pub context_window_size: usize,
    pub resource_check_enabled: bool,
    pub dependency_lookahead: u32, // How many levels to look ahead for dependencies
}

/// Task selection result
#[derive(Debug, Clone)]
pub struct TaskSelection {
    pub task_id: TaskId,
    pub score: f64,
    pub selection_reason: String,
    pub estimated_resources: ResourceRequirement,
}

/// Resource requirements for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirement {
    pub memory_mb: u64,
    pub cpu_percent: f64,
    pub estimated_duration: Duration,
    pub concurrent_task_limit: u32,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            scoring_weights: ScoringWeights::default(),
            resource_monitor: ResourceMonitor::new(),
            context_cache: ContextCache::new(),
            config,
        }
    }

    /// Create scheduler with custom weights
    pub fn with_weights(mut self, weights: ScoringWeights) -> Self {
        self.scoring_weights = weights;
        self
    }

    /// Select the next task to execute
    pub async fn select_next_task(&self, tree: &TaskTree) -> Option<TaskSelection> {
        let eligible_tasks = self.get_eligible_tasks(tree).await;

        if eligible_tasks.is_empty() {
            debug!("No eligible tasks found");
            return None;
        }

        let scored_tasks = eligible_tasks
            .iter()
            .map(|&task_id| {
                let score = self.calculate_task_score(task_id, tree);
                let task = tree.get_task(task_id).unwrap();
                let estimated_resources = self.estimate_task_resources(task);

                TaskSelection {
                    task_id,
                    score,
                    selection_reason: self.build_selection_reason(task_id, score, tree),
                    estimated_resources,
                }
            })
            .collect::<Vec<_>>();

        // Select task using weighted random selection or pure scoring
        let selection = if self.config.selection_randomization > 0.0 {
            self.weighted_random_selection(scored_tasks).await
        } else {
            scored_tasks.into_iter().max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        };

        if let Some(ref selection) = selection {
            info!(
                "Selected task {} with score {:.2}: {}",
                selection.task_id, selection.score, selection.selection_reason
            );
        }

        selection
    }

    /// Get tasks eligible for execution
    async fn get_eligible_tasks(&self, tree: &TaskTree) -> Vec<TaskId> {
        let mut eligible = Vec::new();

        for &task_id in &tree.get_all_task_ids() {
            if self.is_task_eligible(task_id, tree).await {
                eligible.push(task_id);
            }
        }

        eligible
    }

    /// Check if a task is eligible for execution
    async fn is_task_eligible(&self, task_id: TaskId, tree: &TaskTree) -> bool {
        let task = match tree.get_task(task_id) {
            Ok(task) => task,
            Err(_) => return false,
        };

        // Check basic status eligibility
        if !task.is_runnable() {
            return false;
        }

        // Check all dependencies are satisfied
        if let Ok(deps_satisfied) = tree.are_dependencies_satisfied(task_id) {
            if !deps_satisfied {
                return false;
            }
        } else {
            return false;
        }

        // Check resource availability if enabled
        if self.config.resource_check_enabled && !self.resource_monitor.can_execute_task(task).await
        {
            return false;
        }

        // Check for exclusive dependencies
        if self.has_conflicting_active_tasks(task, tree).await {
            return false;
        }

        true
    }

    /// Calculate comprehensive task score
    fn calculate_task_score(&self, task_id: TaskId, tree: &TaskTree) -> f64 {
        let task = tree.get_task(task_id).unwrap();

        let priority_score = self.calculate_priority_score(task);
        let dependency_score = self.calculate_dependency_score(task, tree);
        let context_score = self.calculate_context_score(task);
        let resource_score = self.calculate_resource_score(task);
        let history_score = self.calculate_history_score(task);
        let age_score = self.calculate_age_score(task);
        let complexity_score = self.calculate_complexity_score(task);

        let weights = &self.scoring_weights;

        let total_score = priority_score * weights.priority_weight
            + dependency_score * weights.dependency_weight
            + context_score * weights.context_similarity_weight
            + resource_score * weights.resource_availability_weight
            + history_score * weights.failure_penalty_weight
            + age_score * weights.age_bonus_weight
            + complexity_score * weights.complexity_weight;

        // Normalize score to 0-100 range
        total_score.clamp(0.0, 100.0)
    }

    /// Calculate priority-based score
    fn calculate_priority_score(&self, task: &Task) -> f64 {
        task.priority_value() as f64
    }

    /// Calculate dependency-based score (higher for tasks that unblock others)
    fn calculate_dependency_score(&self, task: &Task, tree: &TaskTree) -> f64 {
        let mut score = 0.0;

        // Count how many tasks depend on this one
        let dependent_count = tree
            .tasks
            .values()
            .filter(|t| t.dependencies.contains(&task.id))
            .count() as f64;

        score += dependent_count * 2.0; // Each dependent task adds 2 points

        // Bonus for tasks that are critical path items
        if self.is_on_critical_path(task, tree) {
            score += 5.0;
        }

        score
    }

    /// Calculate context similarity score
    fn calculate_context_score(&self, task: &Task) -> f64 {
        let task_files: std::collections::HashSet<_> =
            task.metadata.file_refs.iter().map(|f| &f.path).collect();

        let recent_files: std::collections::HashSet<_> =
            self.context_cache.recent_files.iter().collect();

        if task_files.is_empty() || recent_files.is_empty() {
            return 0.0;
        }

        let intersection = task_files.intersection(&recent_files).count() as f64;
        let union = task_files.union(&recent_files).count() as f64;

        if union > 0.0 {
            (intersection / union) * 10.0 // Scale to 0-10 range
        } else {
            0.0
        }
    }

    /// Calculate resource availability score
    fn calculate_resource_score(&self, task: &Task) -> f64 {
        let requirements = self.estimate_task_resources(task);

        // Score based on how well current resources match requirements
        let memory_ratio = self.resource_monitor.current_usage.max_memory_mb as f64
            / (requirements.memory_mb as f64).max(1.0);
        let cpu_ratio = self.resource_monitor.current_usage.cpu_time_seconds
            / requirements.cpu_percent.max(0.1);

        // Lower resource usage = higher score
        let resource_efficiency = (2.0 - memory_ratio.min(2.0)) + (2.0 - cpu_ratio.min(2.0));
        resource_efficiency * 2.5 // Scale to 0-10 range
    }

    /// Calculate history-based score (penalty for previous failures)
    fn calculate_history_score(&self, task: &Task) -> f64 {
        let mut score = 0.0;

        let failure_count = task
            .execution_history
            .iter()
            .filter(|record| matches!(record.status, TaskStatus::Failed { .. }))
            .count() as f64;

        // Penalty for failures, but with diminishing returns
        score -= (failure_count * 2.0).min(8.0);

        // Bonus for successful completions in similar tasks
        let success_count = task
            .execution_history
            .iter()
            .filter(|record| matches!(record.status, TaskStatus::Completed { .. }))
            .count() as f64;

        score += success_count * 1.0;

        score
    }

    /// Calculate age-based score (older tasks get bonus)
    fn calculate_age_score(&self, task: &Task) -> f64 {
        let age_hours = task.age().num_hours() as f64;

        // Logarithmic bonus for age to prevent starvation
        if age_hours > 0.0 {
            (age_hours + 1.0).ln() * 2.0
        } else {
            0.0
        }
    }

    /// Calculate complexity-based score
    fn calculate_complexity_score(&self, task: &Task) -> f64 {
        match &task.metadata.estimated_complexity {
            Some(complexity) => {
                // Prefer moderate complexity tasks - not too simple, not too complex
                match complexity {
                    ComplexityLevel::Trivial => 2.0,
                    ComplexityLevel::Simple => 4.0,
                    ComplexityLevel::Moderate => 5.0, // Optimal
                    ComplexityLevel::Complex => 3.0,
                    ComplexityLevel::Epic => 1.0,
                }
            }
            None => 3.0, // Default score for unknown complexity
        }
    }

    /// Check if task is on the critical path
    fn is_on_critical_path(&self, task: &Task, tree: &TaskTree) -> bool {
        // Simplified critical path detection
        // In practice, this would use more sophisticated graph analysis
        let dependent_count = tree
            .tasks
            .values()
            .filter(|t| t.dependencies.contains(&task.id))
            .count();

        dependent_count > 2 || task.metadata.priority == TaskPriority::Critical
    }

    /// Estimate resource requirements for a task
    fn estimate_task_resources(&self, task: &Task) -> ResourceRequirement {
        let base_memory = 256; // MB
        let base_cpu = 10.0; // percent

        let complexity_multiplier = match task.metadata.estimated_complexity {
            Some(ComplexityLevel::Trivial) => 0.5,
            Some(ComplexityLevel::Simple) => 1.0,
            Some(ComplexityLevel::Moderate) => 2.0,
            Some(ComplexityLevel::Complex) => 4.0,
            Some(ComplexityLevel::Epic) => 8.0,
            None => 1.5,
        };

        let duration = task.metadata.estimated_duration.unwrap_or_else(|| {
            match task.metadata.estimated_complexity {
                Some(ref complexity) => complexity.estimated_duration(),
                None => Duration::minutes(30),
            }
        });

        ResourceRequirement {
            memory_mb: (base_memory as f64 * complexity_multiplier) as u64,
            cpu_percent: base_cpu * complexity_multiplier,
            estimated_duration: duration,
            concurrent_task_limit: match task.metadata.estimated_complexity {
                Some(ComplexityLevel::Epic) | Some(ComplexityLevel::Complex) => 1,
                _ => 3,
            },
        }
    }

    /// Check for conflicting active tasks
    async fn has_conflicting_active_tasks(&self, _task: &Task, _tree: &TaskTree) -> bool {
        // Placeholder for conflict detection
        // Would check for resource conflicts, file locks, etc.
        false
    }

    /// Weighted random selection from scored tasks
    async fn weighted_random_selection(
        &self,
        mut selections: Vec<TaskSelection>,
    ) -> Option<TaskSelection> {
        if selections.is_empty() {
            return None;
        }

        // Sort by score for debugging
        selections.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let randomization = self.config.selection_randomization;

        if randomization <= 0.0 {
            // Pure scoring - return highest scored task
            return selections.into_iter().next();
        }

        if randomization >= 1.0 {
            // Pure random - ignore scores
            let mut rng = thread_rng();
            return selections.choose(&mut rng).cloned();
        }

        // Weighted selection based on scores
        let total_score: f64 = selections.iter().map(|s| s.score.max(0.1)).sum();
        let mut rng = thread_rng();
        let random_value = rng.gen_range(0.0..total_score);

        let mut cumulative_score = 0.0;
        for selection in selections {
            cumulative_score += selection.score.max(0.1);
            if cumulative_score >= random_value {
                return Some(selection);
            }
        }

        None
    }

    /// Build a human-readable explanation for task selection
    fn build_selection_reason(&self, task_id: TaskId, score: f64, tree: &TaskTree) -> String {
        let task = tree.get_task(task_id).unwrap();

        let mut reasons = Vec::new();

        if task.priority_value() >= 8 {
            reasons.push(format!(
                "high priority ({})",
                task.metadata.priority.clone() as u8
            ));
        }

        let dependent_count = tree
            .tasks
            .values()
            .filter(|t| t.dependencies.contains(&task.id))
            .count();

        if dependent_count > 0 {
            reasons.push(format!("blocks {} other tasks", dependent_count));
        }

        let age_hours = task.age().num_hours();
        if age_hours > 24 {
            reasons.push(format!("aged {} hours", age_hours));
        }

        if let Some(complexity) = &task.metadata.estimated_complexity {
            if matches!(
                complexity,
                ComplexityLevel::Trivial | ComplexityLevel::Simple
            ) {
                reasons.push("quick win".to_string());
            }
        }

        if reasons.is_empty() {
            format!("score {:.1}", score)
        } else {
            reasons.join(", ")
        }
    }

    /// Update context cache with recent activity
    pub fn update_context(&mut self, files: Vec<std::path::PathBuf>, repositories: Vec<String>) {
        self.context_cache.recent_files = files;
        self.context_cache.active_repositories = repositories;
        self.context_cache.last_updated = Utc::now();

        // Limit cache size
        if self.context_cache.recent_files.len() > self.config.context_window_size {
            self.context_cache
                .recent_files
                .truncate(self.config.context_window_size);
        }
    }
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            priority_weight: 10.0,
            dependency_weight: 8.0,
            context_similarity_weight: 5.0,
            resource_availability_weight: 3.0,
            failure_penalty_weight: -2.0,
            age_bonus_weight: 2.0,
            complexity_weight: 1.0,
        }
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 3,
            selection_randomization: 0.1, // Slight randomization
            context_window_size: 50,
            resource_check_enabled: true,
            dependency_lookahead: 3,
        }
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            max_concurrent_tasks: 3,
            memory_limit_mb: 8192,
            cpu_limit_percent: 80.0,
            current_usage: ResourceUsage {
                max_memory_mb: 0,
                cpu_time_seconds: 0.0,
                disk_io_mb: 0,
                network_requests: 0,
            },
        }
    }

    pub async fn can_execute_task(&self, task: &Task) -> bool {
        // Simplified resource check
        let _requirements = self.estimate_requirements(task);

        // In practice, would check current system usage
        true
    }

    fn estimate_requirements(&self, task: &Task) -> ResourceRequirement {
        let scheduler = TaskScheduler::new(SchedulerConfig::default());
        scheduler.estimate_task_resources(task)
    }
}

impl Default for ContextCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextCache {
    pub fn new() -> Self {
        Self {
            recent_files: Vec::new(),
            active_repositories: Vec::new(),
            claude_context_window: Vec::new(),
            last_updated: Utc::now(),
        }
    }
}
