use crate::task::types::*;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, error, info};

/// Task execution context with all necessary state
#[derive(Debug, Clone)]
pub struct TaskExecutionContext {
    pub task_id: TaskId,
    pub claude_session_id: Option<ClaudeSessionId>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub file_watchers: Vec<String>, // Simplified file watcher representation
    pub resource_allocation: ResourceAllocation,
    pub execution_start: DateTime<Utc>,
    pub timeout: Option<Duration>,
}

/// Resource allocation for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub max_memory_mb: u64,
    pub max_cpu_percent: f64,
    pub max_duration: Duration,
    pub temp_storage_mb: u64,
    pub network_bandwidth_mbps: f64,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskExecutionResult {
    /// Task completed successfully
    Completed {
        result: serde_json::Value,
        files_modified: Vec<PathBuf>,
        build_artifacts: Vec<PathBuf>,
        execution_metrics: ExecutionMetrics,
    },
    /// Task completed but created subtasks
    CompletedWithSubtasks {
        result: serde_json::Value,
        subtasks: Vec<TaskSpec>,
        files_modified: Vec<PathBuf>,
        execution_metrics: ExecutionMetrics,
    },
    /// Task is blocked and cannot proceed
    Blocked {
        reason: String,
        required_resources: Vec<String>,
        retry_after: Option<Duration>,
        partial_progress: Option<serde_json::Value>,
    },
    /// Task failed with error
    Failed {
        error: TaskError,
        partial_progress: Option<serde_json::Value>,
        recovery_suggestions: Vec<String>,
    },
}

/// Execution metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub duration: Duration,
    pub memory_used_mb: u64,
    pub cpu_time_seconds: f64,
    pub files_read: u32,
    pub files_written: u32,
    pub api_calls_made: u32,
    pub tokens_consumed: u32,
}

/// Task executor that handles the actual execution logic
pub struct TaskExecutor {
    config: ExecutorConfig,
    resource_limits: ResourceAllocation,
}

/// Configuration for task executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub default_timeout: Duration,
    pub max_retries: u32,
    pub enable_monitoring: bool,
    pub workspace_root: PathBuf,
    pub temp_dir: PathBuf,
}

/// Mock Claude Code interface for now
#[async_trait::async_trait]
pub trait ClaudeCodeInterface: Send + Sync {
    async fn execute_task_with_context(
        &self,
        prompt: String,
        context: &TaskExecutionContext,
    ) -> Result<String>;

    async fn create_session(&self) -> Result<ClaudeSessionId>;
    async fn close_session(&self, session_id: ClaudeSessionId) -> Result<()>;
}

/// Simple mock implementation
pub struct MockClaudeInterface;

#[async_trait::async_trait]
impl ClaudeCodeInterface for MockClaudeInterface {
    async fn execute_task_with_context(
        &self,
        prompt: String,
        _context: &TaskExecutionContext,
    ) -> Result<String> {
        // Mock implementation - just returns a success message
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(format!(
            "Task executed successfully with prompt: {}",
            prompt.chars().take(50).collect::<String>()
        ))
    }

    async fn create_session(&self) -> Result<ClaudeSessionId> {
        Ok(uuid::Uuid::new_v4())
    }

    async fn close_session(&self, _session_id: ClaudeSessionId) -> Result<()> {
        Ok(())
    }
}

impl TaskExecutor {
    /// Create a new task executor
    pub fn new(config: ExecutorConfig, resource_limits: ResourceAllocation) -> Self {
        Self {
            config,
            resource_limits,
        }
    }

    /// Execute a task with the given context
    pub async fn execute_task(
        &self,
        task: &Task,
        claude_interface: &dyn ClaudeCodeInterface,
    ) -> Result<TaskExecutionResult> {
        let execution_start = Utc::now();

        // Set up execution context
        let context = self
            .prepare_execution_context(task, execution_start)
            .await?;

        info!("Starting execution of task {}: {}", task.id, task.title);

        // Execute the task through Claude Code
        let result = match self
            .execute_with_claude(task, &context, claude_interface)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                error!("Task execution failed: {}", error);
                return Ok(TaskExecutionResult::Failed {
                    error: TaskError::Other {
                        message: error.to_string(),
                        source: Some("TaskExecutor".to_string()),
                    },
                    partial_progress: None,
                    recovery_suggestions: vec![
                        "Check task requirements".to_string(),
                        "Verify dependencies".to_string(),
                    ],
                });
            }
        };

        // Calculate execution metrics
        let duration = Utc::now().signed_duration_since(execution_start);
        let metrics = ExecutionMetrics {
            duration,
            memory_used_mb: 128, // Mock values
            cpu_time_seconds: duration.num_seconds() as f64 * 0.1,
            files_read: 5,
            files_written: 2,
            api_calls_made: 1,
            tokens_consumed: 1000,
        };

        // Process the execution result
        let execution_result = self.process_claude_response(result, task, metrics).await?;

        info!("Completed execution of task {} in {:?}", task.id, duration);
        Ok(execution_result)
    }

    /// Prepare execution context for a task
    async fn prepare_execution_context(
        &self,
        task: &Task,
        execution_start: DateTime<Utc>,
    ) -> Result<TaskExecutionContext> {
        let working_dir = self.config.workspace_root.join(format!("task_{}", task.id));

        // Create working directory if it doesn't exist
        if !working_dir.exists() {
            tokio::fs::create_dir_all(&working_dir).await?;
        }

        // Prepare environment variables
        let mut environment = HashMap::new();
        environment.insert("TASK_ID".to_string(), task.id.to_string());
        environment.insert("TASK_TITLE".to_string(), task.title.clone());
        environment.insert("WORKING_DIR".to_string(), working_dir.display().to_string());

        // Add task-specific environment variables
        for (key, value) in &task.metadata.context_requirements.environment_vars {
            environment.insert(key.clone(), value.clone());
        }

        let timeout = task
            .metadata
            .estimated_duration
            .or(Some(self.config.default_timeout));

        Ok(TaskExecutionContext {
            task_id: task.id,
            claude_session_id: None,
            working_directory: working_dir,
            environment,
            file_watchers: Vec::new(),
            resource_allocation: self.resource_limits.clone(),
            execution_start,
            timeout,
        })
    }

    /// Execute task through Claude Code interface
    async fn execute_with_claude(
        &self,
        task: &Task,
        context: &TaskExecutionContext,
        claude_interface: &dyn ClaudeCodeInterface,
    ) -> Result<String> {
        // Build task prompt with context
        let prompt = self.build_task_prompt(task, context).await?;

        debug!("Executing task with prompt length: {}", prompt.len());

        // Execute through Claude Code interface
        let response = claude_interface
            .execute_task_with_context(prompt, context)
            .await?;

        Ok(response)
    }

    /// Build a comprehensive prompt for Claude Code
    async fn build_task_prompt(
        &self,
        task: &Task,
        context: &TaskExecutionContext,
    ) -> Result<String> {
        let mut prompt = String::new();

        prompt.push_str(&format!("# Task: {}\n\n", task.title));
        prompt.push_str(&format!("## Description\n{}\n\n", task.description));

        // Add priority and complexity information
        prompt.push_str("## Task Metadata\n");
        prompt.push_str(&format!("- Priority: {:?}\n", task.metadata.priority));
        if let Some(complexity) = &task.metadata.estimated_complexity {
            prompt.push_str(&format!("- Complexity: {:?}\n", complexity));
        }
        prompt.push_str(&format!(
            "- Created: {}\n\n",
            task.created_at.format("%Y-%m-%d %H:%M:%S")
        ));

        // Add file references
        if !task.metadata.file_refs.is_empty() {
            prompt.push_str("## Relevant Files\n");
            for file_ref in &task.metadata.file_refs {
                prompt.push_str(&format!(
                    "- {} ({})\n",
                    file_ref.path.display(),
                    format!("{:?}", file_ref.importance).to_lowercase()
                ));
            }
            prompt.push('\n');
        }

        // Add repository context
        if !task.metadata.repository_refs.is_empty() {
            prompt.push_str("## Repository Context\n");
            for repo_ref in &task.metadata.repository_refs {
                prompt.push_str(&format!("- Repository: {}\n", repo_ref.name));
                if let Some(branch) = &repo_ref.branch {
                    prompt.push_str(&format!("  Branch: {}\n", branch));
                }
            }
            prompt.push('\n');
        }

        // Add context requirements
        let context_reqs = &task.metadata.context_requirements;
        if !context_reqs.is_empty() {
            prompt.push_str("## Context Requirements\n");

            if !context_reqs.required_files.is_empty() {
                prompt.push_str("### Required Files\n");
                for file in &context_reqs.required_files {
                    prompt.push_str(&format!("- {}\n", file.display()));
                }
            }

            if !context_reqs.build_dependencies.is_empty() {
                prompt.push_str("### Build Dependencies\n");
                for dep in &context_reqs.build_dependencies {
                    prompt.push_str(&format!("- {}\n", dep));
                }
            }
            prompt.push('\n');
        }

        // Add working directory information
        prompt.push_str(&format!(
            "## Working Directory\n{}\n\n",
            context.working_directory.display()
        ));

        // Add execution constraints
        prompt.push_str("## Execution Constraints\n");
        prompt.push_str(&format!(
            "- Max Memory: {} MB\n",
            context.resource_allocation.max_memory_mb
        ));
        prompt.push_str(&format!(
            "- Max CPU: {:.1}%\n",
            context.resource_allocation.max_cpu_percent
        ));
        if let Some(timeout) = context.timeout {
            prompt.push_str(&format!("- Timeout: {} minutes\n", timeout.num_minutes()));
        }

        prompt.push_str("\n## Instructions\n");
        prompt.push_str("Please execute this task according to the requirements above. ");
        prompt.push_str("If the task is complex and needs to be broken down, ");
        prompt.push_str("provide subtask specifications in your response. ");
        prompt
            .push_str("Report any files you modify and provide a summary of the work completed.\n");

        Ok(prompt)
    }

    /// Process Claude Code response and determine execution result
    async fn process_claude_response(
        &self,
        response: String,
        task: &Task,
        metrics: ExecutionMetrics,
    ) -> Result<TaskExecutionResult> {
        // Simple response processing - in practice would be more sophisticated
        debug!("Processing Claude response length: {}", response.len());

        // Check for blocking conditions
        if response.to_lowercase().contains("blocked")
            || response.to_lowercase().contains("cannot proceed")
        {
            return Ok(TaskExecutionResult::Blocked {
                reason: "Task reported as blocked by Claude Code".to_string(),
                required_resources: vec!["Manual intervention".to_string()],
                retry_after: Some(Duration::minutes(30)),
                partial_progress: Some(serde_json::json!({"response": response})),
            });
        }

        // Check for subtask creation indicators
        if response.to_lowercase().contains("subtask")
            || response.to_lowercase().contains("break down")
        {
            // In practice, would parse actual subtask specifications from response
            let subtasks = vec![TaskSpec {
                title: format!("Subtask for {}", task.title),
                description: "Auto-generated subtask based on task decomposition".to_string(),
                metadata: TaskMetadata {
                    priority: task.metadata.priority.clone(),
                    estimated_complexity: Some(ComplexityLevel::Simple),
                    estimated_duration: Some(Duration::minutes(15)),
                    repository_refs: task.metadata.repository_refs.clone(),
                    file_refs: Vec::new(),
                    tags: task.metadata.tags.clone(),
                    context_requirements: ContextRequirements::new(),
                },
                dependencies: Vec::new(),
            }];

            return Ok(TaskExecutionResult::CompletedWithSubtasks {
                result: serde_json::json!({
                    "response": response,
                    "subtasks_created": subtasks.len()
                }),
                subtasks,
                files_modified: vec![self.config.workspace_root.join("example_file.rs")],
                execution_metrics: metrics,
            });
        }

        // Default: successful completion
        Ok(TaskExecutionResult::Completed {
            result: serde_json::json!({
                "response": response,
                "status": "completed"
            }),
            files_modified: vec![self.config.workspace_root.join("example_file.rs")],
            build_artifacts: Vec::new(),
            execution_metrics: metrics,
        })
    }

    /// Estimate resource requirements for a task
    pub fn estimate_resource_requirements(&self, task: &Task) -> ResourceAllocation {
        let base_memory = 256;
        let base_cpu = 20.0;

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

        ResourceAllocation {
            max_memory_mb: (base_memory as f64 * complexity_multiplier) as u64,
            max_cpu_percent: (base_cpu * complexity_multiplier).min(100.0),
            max_duration: duration,
            temp_storage_mb: (512.0 * complexity_multiplier) as u64,
            network_bandwidth_mbps: 10.0,
        }
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::hours(1),
            max_retries: 3,
            enable_monitoring: true,
            workspace_root: PathBuf::from("/tmp/claude-agent"),
            temp_dir: PathBuf::from("/tmp"),
        }
    }
}

impl Default for ResourceAllocation {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024,
            max_cpu_percent: 50.0,
            max_duration: Duration::minutes(30),
            temp_storage_mb: 512,
            network_bandwidth_mbps: 10.0,
        }
    }
}

/// Helper function to create a mock executor for testing
pub fn create_mock_executor() -> TaskExecutor {
    TaskExecutor::new(ExecutorConfig::default(), ResourceAllocation::default())
}
