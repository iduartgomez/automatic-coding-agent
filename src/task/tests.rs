#[cfg(test)]
mod tests {
    use crate::task::types::*;
    use crate::task::tree::*;
    use crate::task::manager::*;
    use crate::task::scheduler::*;
    use crate::task::execution::{MockClaudeInterface, TaskExecutionResult};
    use chrono::{Duration, Utc};

    // Helper function to create a task spec for testing
    fn create_test_task_spec() -> TaskSpec {
        TaskSpec {
            title: "Test Task".to_string(),
            description: "A test task for unit testing".to_string(),
            metadata: TaskMetadata {
                priority: TaskPriority::Normal,
                estimated_complexity: Some(ComplexityLevel::Simple),
                estimated_duration: Some(Duration::minutes(15)),
                repository_refs: vec![],
                file_refs: vec![],
                tags: vec!["test".to_string()],
                context_requirements: ContextRequirements::new(),
            },
            dependencies: vec![],
        }
    }

    fn create_test_task() -> Task {
        Task::new(create_test_task_spec(), None)
    }

    #[test]
    fn test_task_creation() {
        let task = create_test_task();

        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, "A test task for unit testing");
        assert!(task.is_runnable());
        assert!(!task.is_terminal());
        assert_eq!(task.metadata.priority, TaskPriority::Normal);
        assert_eq!(task.parent_id, None);
        assert!(task.children.is_empty());
        assert!(task.dependencies.is_empty());
    }

    #[test]
    fn test_task_status_updates() {
        let mut task = create_test_task();

        // Initially pending
        assert!(matches!(task.status, TaskStatus::Pending));
        assert!(task.is_runnable());

        // Update to in progress
        let in_progress_status = TaskStatus::InProgress {
            started_at: Utc::now(),
            estimated_completion: None,
        };
        task.update_status(in_progress_status);
        assert!(task.is_running());
        assert!(!task.is_runnable());

        // Update to completed
        let completed_status = TaskStatus::Completed {
            completed_at: Utc::now(),
            result: TaskResult::Success {
                output: serde_json::json!({"test": "value"}),
                files_created: vec![],
                files_modified: vec![],
                build_artifacts: vec![],
            },
        };
        task.update_status(completed_status);
        assert!(task.is_terminal());
        assert!(!task.is_runnable());
    }

    #[test]
    fn test_task_tree_creation() {
        let tree = TaskTree::new();

        assert_eq!(tree.tasks.len(), 0);
        assert_eq!(tree.roots.len(), 0);
        assert_eq!(tree.metadata.total_tasks_created, 0);
    }

    #[test]
    fn test_task_tree_add_task() {
        let mut tree = TaskTree::new();
        let task = create_test_task();
        let task_id = task.id;

        let result = tree.add_task(task);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), task_id);

        assert_eq!(tree.tasks.len(), 1);
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0], task_id);
        assert_eq!(tree.metadata.total_tasks_created, 1);
    }

    #[test]
    fn test_task_tree_parent_child_relationship() {
        let mut tree = TaskTree::new();

        // Create parent task
        let parent_spec = TaskSpec {
            title: "Parent Task".to_string(),
            description: "Parent task".to_string(),
            metadata: TaskMetadata {
                priority: TaskPriority::High,
                estimated_complexity: Some(ComplexityLevel::Complex),
                estimated_duration: None,
                repository_refs: vec![],
                file_refs: vec![],
                tags: vec![],
                context_requirements: ContextRequirements::new(),
            },
            dependencies: vec![],
        };
        let parent_id = tree.create_task_from_spec(parent_spec, None).unwrap();

        // Create child task
        let child_spec = TaskSpec {
            title: "Child Task".to_string(),
            description: "Child task".to_string(),
            metadata: TaskMetadata {
                priority: TaskPriority::Normal,
                estimated_complexity: Some(ComplexityLevel::Simple),
                estimated_duration: None,
                repository_refs: vec![],
                file_refs: vec![],
                tags: vec![],
                context_requirements: ContextRequirements::new(),
            },
            dependencies: vec![],
        };
        let child_id = tree.create_task_from_spec(child_spec, Some(parent_id)).unwrap();

        // Verify relationships
        let parent = tree.get_task(parent_id).unwrap();
        let child = tree.get_task(child_id).unwrap();

        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0], child_id);
        assert_eq!(child.parent_id, Some(parent_id));
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0], parent_id);
    }

    #[test]
    fn test_task_tree_dependencies() {
        let mut tree = TaskTree::new();

        // Create task A
        let task_a_spec = TaskSpec {
            title: "Task A".to_string(),
            description: "First task".to_string(),
            metadata: TaskMetadata {
                priority: TaskPriority::Normal,
                estimated_complexity: Some(ComplexityLevel::Simple),
                estimated_duration: Some(Duration::minutes(15)),
                repository_refs: vec![],
                file_refs: vec![],
                tags: vec![],
                context_requirements: ContextRequirements::new(),
            },
            dependencies: vec![],
        };
        let task_a_id = tree.create_task_from_spec(task_a_spec, None).unwrap();

        // Create task B that depends on A
        let task_b_spec = TaskSpec {
            title: "Task B".to_string(),
            description: "Second task".to_string(),
            metadata: TaskMetadata {
                priority: TaskPriority::Normal,
                estimated_complexity: Some(ComplexityLevel::Simple),
                estimated_duration: Some(Duration::minutes(15)),
                repository_refs: vec![],
                file_refs: vec![],
                tags: vec![],
                context_requirements: ContextRequirements::new(),
            },
            dependencies: vec![task_a_id],
        };
        let task_b_id = tree.create_task_from_spec(task_b_spec, None).unwrap();

        // Initially, A should be eligible but B should not
        let eligible = tree.get_eligible_tasks();
        assert!(eligible.contains(&task_a_id));
        assert!(!eligible.contains(&task_b_id));

        // Complete task A
        tree.update_task_status(task_a_id, TaskStatus::Completed {
            completed_at: Utc::now(),
            result: TaskResult::Success {
                output: serde_json::json!({}),
                files_created: vec![],
                files_modified: vec![],
                build_artifacts: vec![],
            },
        }).unwrap();

        // Now B should be eligible
        let eligible = tree.get_eligible_tasks();
        assert!(!eligible.contains(&task_a_id)); // A is completed
        assert!(eligible.contains(&task_b_id));   // B is now eligible
    }

    #[tokio::test]
    async fn test_task_manager_creation() {
        let config = TaskManagerConfig::default();
        let manager = TaskManager::new(config);

        let progress = manager.get_progress().await.unwrap();
        assert_eq!(progress.total_tasks, 0);
        assert_eq!(progress.completed_tasks, 0);
    }

    #[tokio::test]
    async fn test_task_manager_create_task() {
        let config = TaskManagerConfig::default();
        let manager = TaskManager::new(config);

        let spec = TaskSpec {
            title: "Manager Test Task".to_string(),
            description: "Test task for manager".to_string(),
            metadata: TaskMetadata {
                priority: TaskPriority::High,
                estimated_complexity: Some(ComplexityLevel::Moderate),
                estimated_duration: Some(Duration::minutes(30)),
                repository_refs: vec![],
                file_refs: vec![],
                tags: vec!["manager-test".to_string()],
                context_requirements: ContextRequirements::new(),
            },
            dependencies: vec![],
        };

        let task_id = manager.create_task(spec, None).await.unwrap();
        let task = manager.get_task(task_id).await.unwrap();

        assert_eq!(task.title, "Manager Test Task");
        assert_eq!(task.metadata.priority, TaskPriority::High);
        assert!(task.is_runnable());
    }

    #[tokio::test]
    async fn test_task_scheduler_selection() {
        let mut config = SchedulerConfig::default();
        config.selection_randomization = 0.0; // Pure scoring, no randomization
        let scheduler = TaskScheduler::new(config);

        let mut tree = TaskTree::new();

        // Create tasks with different priorities
        let high_priority_spec = TaskSpec {
            title: "High Priority Task".to_string(),
            description: "High priority task".to_string(),
            metadata: TaskMetadata {
                priority: TaskPriority::High,
                estimated_complexity: Some(ComplexityLevel::Simple),
                estimated_duration: Some(Duration::minutes(10)),
                repository_refs: vec![],
                file_refs: vec![],
                tags: vec![],
                context_requirements: ContextRequirements::new(),
            },
            dependencies: vec![],
        };
        let high_priority_id = tree.create_task_from_spec(high_priority_spec, None).unwrap();

        let low_priority_spec = TaskSpec {
            title: "Low Priority Task".to_string(),
            description: "Low priority task".to_string(),
            metadata: TaskMetadata {
                priority: TaskPriority::Low,
                estimated_complexity: Some(ComplexityLevel::Simple),
                estimated_duration: Some(Duration::minutes(10)),
                repository_refs: vec![],
                file_refs: vec![],
                tags: vec![],
                context_requirements: ContextRequirements::new(),
            },
            dependencies: vec![],
        };
        let _low_priority_id = tree.create_task_from_spec(low_priority_spec, None).unwrap();

        // Scheduler should prefer high priority task
        let selection = scheduler.select_next_task(&tree).await.unwrap();

        // Check that a high priority task was selected (not necessarily the specific ID)
        let selected_task = tree.get_task(selection.task_id).unwrap();
        assert_eq!(selected_task.metadata.priority, TaskPriority::High);
        assert!(selection.score > 0.0);
    }

    #[tokio::test]
    async fn test_task_execution() {
        use crate::task::execution::{TaskExecutor, ExecutorConfig, ResourceAllocation};

        let config = ExecutorConfig::default();
        let resources = ResourceAllocation::default();
        let executor = TaskExecutor::new(config, resources);
        let claude_interface = MockClaudeInterface;

        let task = create_test_task();
        let result = executor.execute_task(&task, &claude_interface).await.unwrap();

        match result {
            TaskExecutionResult::Completed { result: _, files_modified, execution_metrics, .. } => {
                assert!(!files_modified.is_empty());
                assert!(execution_metrics.duration.num_seconds() >= 0);
            }
            _ => panic!("Expected task to complete successfully"),
        }
    }

    #[test]
    fn test_context_requirements_merge() {
        let mut context1 = ContextRequirements::new();
        context1.required_files.push(std::path::PathBuf::from("file1.rs"));
        context1.build_dependencies.push("dep1".to_string());

        let mut context2 = ContextRequirements::new();
        context2.required_files.push(std::path::PathBuf::from("file2.rs"));
        context2.build_dependencies.push("dep2".to_string());
        context2.environment_vars.insert("KEY".to_string(), "value".to_string());

        context1.merge_with(&context2);

        assert_eq!(context1.required_files.len(), 2);
        assert_eq!(context1.build_dependencies.len(), 2);
        assert_eq!(context1.environment_vars.len(), 1);
        assert!(context1.required_files.contains(&std::path::PathBuf::from("file1.rs")));
        assert!(context1.required_files.contains(&std::path::PathBuf::from("file2.rs")));
    }

    #[test]
    fn test_complexity_level_values() {
        assert_eq!(ComplexityLevel::Trivial.value(), 0);
        assert_eq!(ComplexityLevel::Simple.value(), 1);
        assert_eq!(ComplexityLevel::Moderate.value(), 2);
        assert_eq!(ComplexityLevel::Complex.value(), 3);
        assert_eq!(ComplexityLevel::Epic.value(), 4);

        assert!(ComplexityLevel::Trivial.estimated_duration() < ComplexityLevel::Epic.estimated_duration());
    }

    #[test]
    fn test_task_priority_values() {
        assert_eq!(TaskPriority::Critical.value(), 10);
        assert_eq!(TaskPriority::High.value(), 8);
        assert_eq!(TaskPriority::Normal.value(), 5);
        assert_eq!(TaskPriority::Low.value(), 3);
        assert_eq!(TaskPriority::Background.value(), 1);

        assert!(TaskPriority::Critical > TaskPriority::Low);
    }
}

