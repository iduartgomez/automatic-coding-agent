use crate::claude::{ClaudeCodeInterface, ClaudeConfig};
use crate::session::{SessionInitOptions, SessionManager, SessionManagerConfig};
use crate::task::{TaskManager, TaskManagerConfig, TaskSpec, TaskStatus};
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

/// Integrated agent system that combines task management, session persistence, and Claude integration
pub struct AgentSystem {
    task_manager: Arc<TaskManager>,
    session_manager: Arc<SessionManager>,
    claude_interface: Arc<ClaudeCodeInterface>,
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub workspace_path: std::path::PathBuf,
    pub session_config: SessionManagerConfig,
    pub task_config: TaskManagerConfig,
    pub claude_config: ClaudeConfig,
}

impl AgentSystem {
    pub async fn new(config: AgentConfig) -> Result<Self> {
        // Initialize session manager
        let session_manager = Arc::new(
            SessionManager::new(
                config.workspace_path,
                config.session_config,
                SessionInitOptions::default(),
            )
            .await?,
        );

        // Initialize task manager
        let task_manager = Arc::new(TaskManager::new(config.task_config));

        // Initialize Claude interface
        let claude_interface = Arc::new(
            ClaudeCodeInterface::new(config.claude_config)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize Claude interface: {}", e))?,
        );

        Ok(Self {
            task_manager,
            session_manager,
            claude_interface,
        })
    }

    /// Process a single task with Claude integration and full persistence
    pub async fn process_task(&self, task_id: Uuid) -> Result<()> {
        // Get task from task manager
        let task = self.task_manager.get_task(task_id).await?;

        tracing::info!("Processing task: {} - {}", task.id, task.title);

        // Update task status to in progress
        self.task_manager
            .update_task_status(
                task_id,
                TaskStatus::InProgress {
                    started_at: chrono::Utc::now(),
                    estimated_completion: None,
                },
            )
            .await?;

        // Save current state
        self.save_session_state().await?;

        // Process with Claude
        let result = self.claude_interface.process_task(&task).await;

        match result {
            Ok(completed_task) => {
                // Update task status to completed
                self.task_manager
                    .update_task_status(task_id, completed_task.status)
                    .await?;

                // Save session state
                self.save_session_state().await?;

                tracing::info!("Task completed successfully: {}", task_id);
                Ok(())
            }
            Err(e) => {
                // Mark task as failed
                self.task_manager
                    .update_task_status(
                        task_id,
                        TaskStatus::Failed {
                            failed_at: chrono::Utc::now(),
                            error: crate::task::types::TaskError::ClaudeError {
                                message: format!("Failed: {}", e),
                                error_code: None,
                                retry_possible: true,
                            },
                            retry_count: 0,
                        },
                    )
                    .await?;

                self.save_session_state().await?;

                tracing::error!("Task failed: {} - {}", task_id, e);
                Err(anyhow::anyhow!("Task processing failed: {}", e))
            }
        }
    }

    /// Create and process a new task
    pub async fn create_and_process_task(&self, title: &str, description: &str) -> Result<Uuid> {
        // Create task spec
        let task_spec = TaskSpec {
            title: title.to_string(),
            description: description.to_string(),
            dependencies: Vec::new(),
            metadata: crate::task::types::TaskMetadata {
                priority: crate::task::types::TaskPriority::Normal,
                estimated_complexity: Some(crate::task::types::ComplexityLevel::Moderate),
                estimated_duration: Some(
                    chrono::Duration::from_std(std::time::Duration::from_secs(300)).unwrap(),
                ),
                repository_refs: Vec::new(),
                file_refs: Vec::new(),
                tags: vec!["auto-generated".to_string()],
                context_requirements: crate::task::types::ContextRequirements {
                    required_files: Vec::new(),
                    required_repositories: Vec::new(),
                    build_dependencies: Vec::new(),
                    environment_vars: std::collections::HashMap::new(),
                    claude_context_keys: Vec::new(),
                },
            },
        };

        // Create task in task manager
        let task_id = self.task_manager.create_task(task_spec, None).await?;

        // Save state
        self.save_session_state().await?;

        // Process the task
        self.process_task(task_id).await?;

        Ok(task_id)
    }

    /// Get system status
    pub async fn get_system_status(&self) -> Result<SystemStatus> {
        let task_stats = self.task_manager.get_statistics().await?;
        let claude_status = self.claude_interface.get_interface_status().await;
        let session_stats = self.session_manager.get_session_statistics().await?;

        Ok(SystemStatus {
            task_stats: task_stats.clone(),
            claude_status: claude_status.clone(),
            session_stats,
            is_healthy: claude_status.is_healthy && task_stats.total_tasks > 0,
        })
    }

    /// Save current session state
    async fn save_session_state(&self) -> Result<()> {
        // Save session
        self.session_manager.save_session().await?;
        self.session_manager
            .create_checkpoint("auto_save".to_string())
            .await?;

        Ok(())
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down agent system...");

        // Save final state
        self.save_session_state().await?;

        // Graceful shutdown of session manager
        self.session_manager.shutdown().await?;

        tracing::info!("Agent system shutdown complete");
        Ok(())
    }

    // Getters for individual components
    pub fn task_manager(&self) -> Arc<TaskManager> {
        self.task_manager.clone()
    }

    pub fn session_manager(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }

    pub fn claude_interface(&self) -> Arc<ClaudeCodeInterface> {
        self.claude_interface.clone()
    }
}

#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub task_stats: crate::task::TaskTreeStatistics,
    pub claude_status: crate::claude::interface::ClaudeInterfaceStatus,
    pub session_stats: crate::session::SessionStatistics,
    pub is_healthy: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        // Create a proper test/temp directory instead of using current dir
        let workspace_path = std::env::temp_dir().join("automatic-coding-agent");

        Self {
            workspace_path,
            session_config: SessionManagerConfig::default(),
            task_config: TaskManagerConfig::default(),
            claude_config: ClaudeConfig::default(),
        }
    }
}
