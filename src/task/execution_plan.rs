//! Execution plan abstraction for unified task processing.
//!
//! This module provides a unified interface for task execution that consolidates
//! both simple task lists and structured configurations into a common execution model.
//!
//! ## Architecture
//!
//! The ExecutionPlan serves as an intermediate representation that bridges:
//! - TaskLoader parsed tasks (simple file-based tasks)
//! - Structured TOML configurations with setup commands
//! - Future execution models (parallel, conditional, dependency-aware)
//!
//! ## Key Components
//!
//! - **ExecutionPlan**: Complete execution specification with setup and tasks
//! - **ExecutionMode**: Sequential vs parallel execution strategy
//! - **PlanMetadata**: Execution context and requirements
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use aca::task::{ExecutionPlan, TaskSpec, SetupCommand};
//!
//! // Create execution plan from simple tasks
//! let plan = ExecutionPlan::new()
//!     .with_task(TaskSpec::new("Implement feature", "Add new functionality"))
//!     .with_sequential_execution();
//!
//! // Create execution plan with setup
//! let plan = ExecutionPlan::new()
//!     .with_setup_command(SetupCommand::new("install_deps", "npm install"))
//!     .with_task(TaskSpec::new("Run tests", "Execute test suite"));
//! ```

use crate::task::{SetupCommand, TaskSpec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Execution strategy for processing tasks within a plan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ExecutionMode {
    /// Execute tasks one by one in order
    #[default]
    Sequential,
    /// Execute tasks in parallel where possible (respecting dependencies)
    Parallel { max_concurrent: Option<usize> },
    /// Execute with intelligent scheduling based on task metadata
    Intelligent,
}

/// Metadata about the execution plan
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanMetadata {
    /// Human-readable name for the execution plan
    pub name: Option<String>,
    /// Description of what this plan accomplishes
    pub description: Option<String>,
    /// Tags for categorization and filtering
    pub tags: Vec<String>,
    /// Expected total execution time
    pub estimated_duration: Option<chrono::Duration>,
    /// Additional key-value metadata
    pub custom_metadata: HashMap<String, String>,
}

/// Unified execution plan containing setup commands and task specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Commands to execute before any task processing (environment setup, etc.)
    pub setup_commands: Vec<SetupCommand>,
    /// Task specifications to be processed by the agent system
    pub task_specs: Vec<TaskSpec>,
    /// How tasks should be executed
    pub execution_mode: ExecutionMode,
    /// Additional metadata about the execution plan
    pub metadata: PlanMetadata,
}

impl ExecutionPlan {
    /// Create a new empty execution plan
    pub fn new() -> Self {
        Self {
            setup_commands: Vec::new(),
            task_specs: Vec::new(),
            execution_mode: ExecutionMode::default(),
            metadata: PlanMetadata::default(),
        }
    }

    /// Add a setup command to the plan
    pub fn with_setup_command(mut self, command: SetupCommand) -> Self {
        self.setup_commands.push(command);
        self
    }

    /// Add multiple setup commands to the plan
    pub fn with_setup_commands(mut self, commands: Vec<SetupCommand>) -> Self {
        self.setup_commands.extend(commands);
        self
    }

    /// Add a task specification to the plan
    pub fn with_task(mut self, task_spec: TaskSpec) -> Self {
        self.task_specs.push(task_spec);
        self
    }

    /// Add multiple task specifications to the plan
    pub fn with_tasks(mut self, tasks: Vec<TaskSpec>) -> Self {
        self.task_specs.extend(tasks);
        self
    }

    /// Set the execution mode for the plan
    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Set sequential execution mode (default)
    pub fn with_sequential_execution(self) -> Self {
        self.with_execution_mode(ExecutionMode::Sequential)
    }

    /// Set parallel execution mode with optional concurrency limit
    pub fn with_parallel_execution(self, max_concurrent: Option<usize>) -> Self {
        self.with_execution_mode(ExecutionMode::Parallel { max_concurrent })
    }

    /// Set intelligent execution mode (uses task metadata for scheduling)
    pub fn with_intelligent_execution(self) -> Self {
        self.with_execution_mode(ExecutionMode::Intelligent)
    }

    /// Set the plan name and description
    pub fn with_metadata(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.metadata.name = Some(name.into());
        self.metadata.description = Some(description.into());
        self
    }

    /// Add tags to the plan
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.metadata.tags.extend(tags);
        self
    }

    /// Set the estimated duration for the entire plan
    pub fn with_estimated_duration(mut self, duration: chrono::Duration) -> Self {
        self.metadata.estimated_duration = Some(duration);
        self
    }

    /// Check if the plan has any setup commands
    pub fn has_setup_commands(&self) -> bool {
        !self.setup_commands.is_empty()
    }

    /// Check if the plan has any tasks
    pub fn has_tasks(&self) -> bool {
        !self.task_specs.is_empty()
    }

    /// Get the number of tasks in the plan
    pub fn task_count(&self) -> usize {
        self.task_specs.len()
    }

    /// Get the number of setup commands in the plan
    pub fn setup_command_count(&self) -> usize {
        self.setup_commands.len()
    }

    /// Check if this plan is empty (no setup commands or tasks)
    pub fn is_empty(&self) -> bool {
        self.setup_commands.is_empty() && self.task_specs.is_empty()
    }

    /// Get a summary string describing the plan
    pub fn summary(&self) -> String {
        match (self.setup_command_count(), self.task_count()) {
            (0, 0) => "Empty execution plan".to_string(),
            (0, 1) => "1 task".to_string(),
            (0, n) => format!("{} tasks", n),
            (1, 0) => "1 setup command".to_string(),
            (s, 0) => format!("{} setup commands", s),
            (1, 1) => "1 setup command, 1 task".to_string(),
            (1, n) => format!("1 setup command, {} tasks", n),
            (s, 1) => format!("{} setup commands, 1 task", s),
            (s, n) => format!("{} setup commands, {} tasks", s, n),
        }
    }

    /// Validate the execution plan for consistency
    pub fn validate(&self) -> Result<(), String> {
        if self.is_empty() {
            return Err("Execution plan is empty".to_string());
        }

        // Validate setup commands
        for (i, command) in self.setup_commands.iter().enumerate() {
            if command.name.is_empty() {
                return Err(format!("Setup command {} has empty name", i));
            }
            if command.command.is_empty() {
                return Err(format!(
                    "Setup command '{}' has empty command",
                    command.name
                ));
            }
        }

        // Validate task specs
        for (i, task) in self.task_specs.iter().enumerate() {
            if task.title.is_empty() {
                return Err(format!("Task {} has empty title", i));
            }
            if task.description.is_empty() {
                return Err(format!("Task '{}' has empty description", task.title));
            }
        }

        Ok(())
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskMetadata;

    #[test]
    fn test_execution_plan_creation() {
        let plan = ExecutionPlan::new();
        assert!(plan.is_empty());
        assert_eq!(plan.task_count(), 0);
        assert_eq!(plan.setup_command_count(), 0);
        assert_eq!(plan.summary(), "Empty execution plan");
    }

    #[test]
    fn test_execution_plan_with_tasks() {
        let task_spec = TaskSpec {
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            dependencies: Vec::new(),
            metadata: TaskMetadata::default(),
        };

        let plan = ExecutionPlan::new()
            .with_task(task_spec)
            .with_sequential_execution();

        assert!(!plan.is_empty());
        assert!(plan.has_tasks());
        assert!(!plan.has_setup_commands());
        assert_eq!(plan.task_count(), 1);
        assert_eq!(plan.summary(), "1 task");
        assert_eq!(plan.execution_mode, ExecutionMode::Sequential);
    }

    #[test]
    fn test_execution_plan_with_setup_commands() {
        let setup_cmd = SetupCommand::new("test", "echo hello");

        let plan = ExecutionPlan::new()
            .with_setup_command(setup_cmd)
            .with_parallel_execution(Some(2));

        assert!(!plan.is_empty());
        assert!(!plan.has_tasks());
        assert!(plan.has_setup_commands());
        assert_eq!(plan.setup_command_count(), 1);
        assert_eq!(plan.summary(), "1 setup command");
        assert_eq!(
            plan.execution_mode,
            ExecutionMode::Parallel {
                max_concurrent: Some(2)
            }
        );
    }

    #[test]
    fn test_execution_plan_with_metadata() {
        let plan = ExecutionPlan::new()
            .with_metadata("Test Plan", "A test execution plan")
            .with_tags(vec!["test".to_string(), "example".to_string()])
            .with_estimated_duration(chrono::Duration::minutes(30));

        assert_eq!(plan.metadata.name, Some("Test Plan".to_string()));
        assert_eq!(
            plan.metadata.description,
            Some("A test execution plan".to_string())
        );
        assert_eq!(plan.metadata.tags, vec!["test", "example"]);
        assert_eq!(
            plan.metadata.estimated_duration,
            Some(chrono::Duration::minutes(30))
        );
    }

    #[test]
    fn test_execution_plan_validation() {
        // Empty plan should fail validation
        let empty_plan = ExecutionPlan::new();
        assert!(empty_plan.validate().is_err());

        // Valid plan should pass validation
        let valid_plan = ExecutionPlan::new().with_task(TaskSpec {
            title: "Valid Task".to_string(),
            description: "Valid description".to_string(),
            dependencies: Vec::new(),
            metadata: TaskMetadata::default(),
        });
        assert!(valid_plan.validate().is_ok());

        // Plan with empty task title should fail
        let invalid_plan = ExecutionPlan::new().with_task(TaskSpec {
            title: "".to_string(),
            description: "Valid description".to_string(),
            dependencies: Vec::new(),
            metadata: TaskMetadata::default(),
        });
        assert!(invalid_plan.validate().is_err());
    }

    #[test]
    fn test_execution_plan_summary() {
        // Test various combinations of setup commands and tasks
        let plan = ExecutionPlan::new()
            .with_setup_command(SetupCommand::new("setup1", "cmd1"))
            .with_setup_command(SetupCommand::new("setup2", "cmd2"))
            .with_task(TaskSpec {
                title: "Task 1".to_string(),
                description: "First task".to_string(),
                dependencies: Vec::new(),
                metadata: TaskMetadata::default(),
            })
            .with_task(TaskSpec {
                title: "Task 2".to_string(),
                description: "Second task".to_string(),
                dependencies: Vec::new(),
                metadata: TaskMetadata::default(),
            });

        assert_eq!(plan.summary(), "2 setup commands, 2 tasks");
    }

    #[test]
    fn test_execution_modes() {
        assert_eq!(ExecutionMode::default(), ExecutionMode::Sequential);

        let sequential = ExecutionMode::Sequential;
        let parallel = ExecutionMode::Parallel {
            max_concurrent: Some(4),
        };
        let _intelligent = ExecutionMode::Intelligent;

        // Test serialization/deserialization
        let sequential_json = serde_json::to_string(&sequential).unwrap();
        let parsed: ExecutionMode = serde_json::from_str(&sequential_json).unwrap();
        assert_eq!(sequential, parsed);

        let parallel_json = serde_json::to_string(&parallel).unwrap();
        let parsed: ExecutionMode = serde_json::from_str(&parallel_json).unwrap();
        assert_eq!(parallel, parsed);
    }
}
