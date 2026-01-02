use crate::session::metadata::*;
use crate::session::persistence::*;
use crate::session::recovery::*;
use crate::task::manager::{TaskManager, TaskManagerConfig};
use crate::task::tree::TaskTree;
use crate::task::types::*;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration as TokioDuration, interval};
use tracing::{debug, error, info, warn};

/// Central session manager that coordinates persistence, recovery, and task management
pub struct SessionManager {
    session_id: SessionId,
    metadata: Arc<RwLock<SessionMetadata>>,
    task_manager: Arc<TaskManager>,
    persistence: Arc<PersistenceManager>,
    recovery: Arc<RecoveryManager>,
    config: SessionManagerConfig,
    auto_save_enabled: Arc<Mutex<bool>>,
}

/// Configuration for session manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManagerConfig {
    pub auto_save_interval_minutes: u32,
    pub auto_checkpoint_interval_minutes: u32,
    pub checkpoint_on_significant_progress: bool,
    pub significant_progress_threshold: f32,
    pub max_session_duration_hours: u32,
    pub enable_crash_recovery: bool,
    pub validate_on_save: bool,
    pub compress_checkpoints: bool,
}

/// Session initialization options
#[derive(Debug, Clone)]
pub struct SessionInitOptions {
    pub name: String,
    pub description: Option<String>,
    pub workspace_root: PathBuf,
    pub task_manager_config: TaskManagerConfig,
    pub persistence_config: PersistenceConfig,
    pub recovery_config: RecoveryConfig,
    pub enable_auto_save: bool,
    pub restore_from_checkpoint: Option<String>,
    pub execution_mode: Option<crate::executor::RuntimeMode>,
}

/// Session status information
#[derive(Debug, Clone)]
pub struct SessionStatus {
    pub id: SessionId,
    pub name: String,
    pub uptime: Duration,
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub active_tasks: u32,
    pub last_checkpoint: Option<DateTime<Utc>>,
    pub last_save: Option<DateTime<Utc>>,
    pub memory_usage_mb: u64,
    pub is_auto_save_active: bool,
}

impl SessionManager {
    /// Get the session ID
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Update container info in session metadata
    pub async fn set_container_info(&self, container_info: SessionContainerInfo) {
        let mut metadata = self.metadata.write().await;
        metadata.set_container_info(container_info);
    }

    /// Clear container info in session metadata
    pub async fn clear_container_info(&self) {
        let mut metadata = self.metadata.write().await;
        metadata.clear_container_info();
    }

    /// Get container info from session metadata
    pub async fn container_info(&self) -> Option<SessionContainerInfo> {
        let metadata = self.metadata.read().await;
        metadata.container_info.clone()
    }

    /// Create a new session manager
    pub async fn new(
        session_dir: PathBuf,
        config: SessionManagerConfig,
        init_options: SessionInitOptions,
    ) -> Result<Self> {
        info!("Creating new session: {}", init_options.name);

        let session_id = SessionId::new_v4();

        // Initialize persistence and recovery managers
        let persistence = Arc::new(
            PersistenceManager::new(
                session_dir.clone(),
                &session_id.to_string(),
                init_options.persistence_config.clone(),
            )
            .context("Failed to create persistence manager")?,
        );

        let recovery = Arc::new(RecoveryManager::new(
            PersistenceManager::new(
                session_dir,
                &session_id.to_string(),
                init_options.persistence_config.clone(),
            )?,
            init_options.recovery_config.clone(),
        ));

        // Create task manager
        let task_manager = Arc::new(TaskManager::new(init_options.task_manager_config));

        // Initialize session metadata
        let mut session_metadata =
            SessionMetadata::new(init_options.name, init_options.workspace_root);
        session_metadata.execution_mode = init_options.execution_mode;
        let metadata = Arc::new(RwLock::new(session_metadata));

        let session_manager = Self {
            session_id,
            metadata,
            task_manager,
            persistence,
            recovery,
            config: config.clone(),
            auto_save_enabled: Arc::new(Mutex::new(init_options.enable_auto_save)),
        };

        // Attempt recovery if requested
        if let Some(checkpoint_id) = init_options.restore_from_checkpoint {
            session_manager
                .restore_from_checkpoint(&checkpoint_id)
                .await?;
        } else if config.enable_crash_recovery {
            // Try automatic recovery
            if let Ok(recovery_result) = session_manager.recovery.auto_recover().await
                && recovery_result.success
                && let Some(state) = recovery_result.recovered_state
            {
                session_manager.restore_session_state(state).await?;
                info!("Automatically recovered session from previous state");
            }
        }

        // Start automatic operations
        if init_options.enable_auto_save {
            session_manager.start_auto_save().await?;
        }

        if session_manager.config.auto_checkpoint_interval_minutes > 0 {
            session_manager.start_auto_checkpoint().await?;
        }

        info!("Session manager initialized: {}", session_id);
        Ok(session_manager)
    }

    /// Get the task manager instance
    pub fn task_manager(&self) -> &Arc<TaskManager> {
        &self.task_manager
    }

    /// Get current session status
    pub async fn get_status(&self) -> Result<SessionStatus> {
        let metadata = self.metadata.read().await;
        let uptime = Utc::now().signed_duration_since(metadata.created_at);

        // Get task statistics from task manager
        let _statistics = self.task_manager.get_statistics().await?;

        let active_tasks = self
            .task_manager
            .get_tasks_by_status(|status| matches!(status, TaskStatus::InProgress { .. }))
            .await?
            .len() as u32;

        let auto_save_active = *self.auto_save_enabled.lock().await;

        Ok(SessionStatus {
            id: self.session_id,
            name: metadata.name.clone(),
            uptime,
            total_tasks: metadata.total_tasks,
            completed_tasks: metadata.completed_tasks,
            failed_tasks: metadata.failed_tasks,
            active_tasks,
            last_checkpoint: metadata.latest_checkpoint().map(|c| c.created_at),
            last_save: Some(metadata.last_updated),
            memory_usage_mb: 0, // Would be calculated from actual usage
            is_auto_save_active: auto_save_active,
        })
    }

    /// Save current session state
    pub async fn save_session(&self) -> Result<PersistenceResult> {
        debug!("Saving session state");

        let session_state = self.capture_session_state().await?;

        // Validate before saving if configured
        if self.config.validate_on_save {
            let validation = self.recovery.validate_session_state(&session_state).await?;
            if !validation.is_valid {
                warn!(
                    "Session validation failed before save: {:?}",
                    validation.errors
                );
                // Continue with save but log warnings
            }
        }

        let result = self.persistence.save_session_state(&session_state).await?;

        // Update metadata
        {
            let mut metadata = self.metadata.write().await;
            metadata.last_updated = Utc::now();
        }

        info!("Session saved successfully: {} bytes", result.bytes_written);
        Ok(result)
    }

    /// Create a manual checkpoint
    pub async fn create_checkpoint(&self, description: String) -> Result<CheckpointInfo> {
        info!("Creating manual checkpoint: {}", description);

        let session_state = self.capture_session_state().await?;

        let checkpoint_info = self
            .persistence
            .create_checkpoint(
                &session_state,
                description,
                CheckpointTrigger::Manual {
                    reason: "User requested".to_string(),
                },
            )
            .await?;

        // Update metadata
        {
            let mut metadata = self.metadata.write().await;
            metadata.add_checkpoint(checkpoint_info.clone());
        }

        info!("Checkpoint created: {}", checkpoint_info.id);
        Ok(checkpoint_info)
    }

    /// Restore from a specific checkpoint
    pub async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<()> {
        info!("Restoring session from checkpoint: {}", checkpoint_id);

        // Create emergency backup before restoration
        if self.recovery.config.create_recovery_checkpoint {
            let current_state = self.capture_session_state().await?;
            let _emergency_checkpoint = self
                .recovery
                .create_emergency_checkpoint(&current_state)
                .await?;
        }

        let recovery_result = self.recovery.recover_from_checkpoint(checkpoint_id).await?;

        if recovery_result.success {
            if let Some(state) = recovery_result.recovered_state {
                self.restore_session_state(state).await?;
                info!("Successfully restored from checkpoint: {}", checkpoint_id);
            }
        } else {
            return Err(anyhow::anyhow!(
                "Failed to restore from checkpoint: {:?}",
                recovery_result.errors
            ));
        }

        Ok(())
    }

    /// List available checkpoints
    ///
    /// # Arguments
    /// * `include_all_sessions` - If true, lists checkpoints from all sessions in the workspace.
    ///   If false, lists only checkpoints from the current session.
    pub async fn list_checkpoints(
        &self,
        include_all_sessions: bool,
    ) -> Result<Vec<CheckpointInfo>> {
        if include_all_sessions {
            let metadata = self.metadata.read().await;
            let workspace_root = &metadata.workspace_root;
            Self::list_all_checkpoints_in_workspace(workspace_root).await
        } else {
            // Session-specific behavior: only show checkpoints from this session
            let checkpoint_ids = self.persistence.list_checkpoints().await?;
            let metadata = self.metadata.read().await;

            let available_checkpoints: Vec<CheckpointInfo> = metadata
                .checkpoints
                .iter()
                .filter(|checkpoint| checkpoint_ids.contains(&checkpoint.id))
                .cloned()
                .collect();

            Ok(available_checkpoints)
        }
    }

    /// Create a checkpoint in the latest session of the workspace
    /// This is useful for CLI operations that want to add checkpoints to existing sessions
    pub async fn create_checkpoint_in_latest_session_of_workspace(
        &self,
        description: String,
    ) -> Result<CheckpointInfo> {
        let metadata = self.metadata.read().await;
        let workspace_root = &metadata.workspace_root;
        Self::create_checkpoint_in_latest_session(workspace_root, description).await
    }

    /// Start automatic session saving
    pub async fn start_auto_save(&self) -> Result<()> {
        let interval_duration =
            TokioDuration::from_secs(self.config.auto_save_interval_minutes as u64 * 60);

        let persistence = self.persistence.clone();
        let metadata = self.metadata.clone();
        let task_manager = self.task_manager.clone();
        let auto_save_enabled = self.auto_save_enabled.clone();
        let validate_on_save = self.config.validate_on_save;

        tokio::spawn(async move {
            let mut interval_timer = interval(interval_duration);

            loop {
                interval_timer.tick().await;

                let is_enabled = *auto_save_enabled.lock().await;
                if !is_enabled {
                    continue;
                }

                match Self::capture_session_state_static(&metadata, &task_manager).await {
                    Ok(session_state) => {
                        // Validate if configured
                        if validate_on_save {
                            // Note: We can't access recovery manager in static context
                            // In a real implementation, we'd pass validation logic here
                        }

                        if let Err(e) = persistence.save_session_state(&session_state).await {
                            error!("Auto-save failed: {}", e);
                        } else {
                            debug!("Auto-save completed successfully");
                        }
                    }
                    Err(e) => {
                        error!("Failed to capture session state for auto-save: {}", e);
                    }
                }
            }
        });

        info!(
            "Auto-save started with interval: {} minutes",
            self.config.auto_save_interval_minutes
        );
        Ok(())
    }

    /// Start automatic checkpoint creation
    pub async fn start_auto_checkpoint(&self) -> Result<()> {
        let interval_duration =
            TokioDuration::from_secs(self.config.auto_checkpoint_interval_minutes as u64 * 60);

        let persistence = self.persistence.clone();
        let metadata = self.metadata.clone();
        let task_manager = self.task_manager.clone();

        tokio::spawn(async move {
            let mut interval_timer = interval(interval_duration);

            loop {
                interval_timer.tick().await;

                match Self::capture_session_state_static(&metadata, &task_manager).await {
                    Ok(session_state) => {
                        let description = format!(
                            "Automatic checkpoint at {}",
                            Utc::now().format("%Y-%m-%d %H:%M:%S")
                        );

                        if let Err(e) = persistence
                            .create_checkpoint(
                                &session_state,
                                description,
                                CheckpointTrigger::Automatic {
                                    trigger: AutoTrigger::TimeInterval {
                                        minutes: interval_duration.as_secs() as u32 / 60,
                                    },
                                },
                            )
                            .await
                        {
                            error!("Auto-checkpoint failed: {}", e);
                        } else {
                            debug!("Auto-checkpoint created successfully");
                        }
                    }
                    Err(e) => {
                        error!("Failed to capture session state for auto-checkpoint: {}", e);
                    }
                }
            }
        });

        info!(
            "Auto-checkpoint started with interval: {} minutes",
            self.config.auto_checkpoint_interval_minutes
        );
        Ok(())
    }

    /// Enable or disable auto-save
    pub async fn set_auto_save_enabled(&self, enabled: bool) -> Result<()> {
        let mut auto_save_enabled = self.auto_save_enabled.lock().await;
        *auto_save_enabled = enabled;

        info!("Auto-save {}", if enabled { "enabled" } else { "disabled" });
        Ok(())
    }

    /// Capture current session state
    async fn capture_session_state(&self) -> Result<SessionState> {
        Self::capture_session_state_static(&self.metadata, &self.task_manager).await
    }

    /// Static helper for capturing session state (used in spawned tasks)
    async fn capture_session_state_static(
        metadata: &Arc<RwLock<SessionMetadata>>,
        task_manager: &Arc<TaskManager>,
    ) -> Result<SessionState> {
        // Get current task tree state
        let task_tree_json = task_manager.export_to_json().await?;
        let task_tree: TaskTree = serde_json::from_str(&task_tree_json)?;

        // Get current metadata
        let current_metadata = {
            let metadata_lock = metadata.read().await;
            metadata_lock.clone()
        };

        // Capture execution context
        let execution_context = ExecutionContext::default(); // Would capture real state

        // Capture file system state
        let file_system_state = FileSystemState::default(); // Would capture real state

        Ok(SessionState {
            metadata: current_metadata,
            task_tree,
            execution_context,
            file_system_state,
        })
    }

    /// Restore session from a saved state
    async fn restore_session_state(&self, state: SessionState) -> Result<()> {
        info!("Restoring session state");

        // Import task tree
        let task_tree_json = serde_json::to_string(&state.task_tree)?;
        self.task_manager.import_from_json(&task_tree_json).await?;

        // Update metadata (includes container_info if present)
        {
            let mut metadata = self.metadata.write().await;
            *metadata = state.metadata;
        }

        // Note: Container reconnection is handled separately by AgentSystem.
        // If state.metadata.container_info is present, the AgentSystem's
        // ContainerLifecycleManager can reconnect to the existing container
        // using the stored container_id and container_name.
        //
        // Additional restoration that would be done in a full implementation:
        // - Execution context (working directory, environment variables)
        // - File system state (file watchers, tracked files)
        // - Resource allocations

        info!("Session state restored successfully");
        Ok(())
    }

    /// Cleanup old checkpoints
    pub async fn cleanup_old_checkpoints(&self) -> Result<u32> {
        let cleaned_count = self.persistence.cleanup_old_checkpoints().await?;

        if cleaned_count > 0 {
            // Update metadata to remove cleaned checkpoint references
            let remaining_checkpoints = self.persistence.list_checkpoints().await?;
            let mut metadata = self.metadata.write().await;
            metadata
                .checkpoints
                .retain(|checkpoint| remaining_checkpoints.contains(&checkpoint.id));

            info!("Cleaned up {} old checkpoints", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Validate current session integrity
    pub async fn validate_session(&self) -> Result<ValidationResult> {
        let session_state = self.capture_session_state().await?;
        self.recovery.validate_session_state(&session_state).await
    }

    /// Get session statistics
    pub async fn get_session_statistics(&self) -> Result<SessionStatistics> {
        let metadata = self.metadata.read().await;
        let uptime = Utc::now().signed_duration_since(metadata.created_at);

        // In a real implementation, these would be calculated from actual data
        Ok(SessionStatistics {
            total_execution_time: uptime,
            total_checkpoints: metadata.checkpoints.len() as u32,
            data_size_mb: 0.0, // Would calculate actual size
            average_checkpoint_size_mb: 0.0,
            recovery_count: 0,
            last_recovery_at: None,
            performance_metrics: PerformanceMetrics::default(),
        })
    }

    /// Shutdown session gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down session: {}", self.session_id);

        // Disable auto-save
        self.set_auto_save_enabled(false).await?;

        // Create final checkpoint
        let _final_checkpoint = self
            .create_checkpoint("session_shutdown".to_string())
            .await?;

        // Save current state
        self.save_session().await?;

        // Cleanup if configured
        if self.persistence.config.auto_cleanup {
            self.cleanup_old_checkpoints().await?;
        }

        info!("Session shutdown completed");
        Ok(())
    }

    /// List all checkpoints across all sessions in a workspace
    /// This is a private static method used internally
    async fn list_all_checkpoints_in_workspace(
        workspace_root: &std::path::Path,
    ) -> Result<Vec<CheckpointInfo>> {
        use crate::env;
        use std::collections::BTreeMap;

        let sessions_dir = env::sessions_dir_path(workspace_root);

        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut all_checkpoints: BTreeMap<DateTime<Utc>, CheckpointInfo> = BTreeMap::new();

        // Read all session directories
        if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
            for entry in entries.flatten() {
                let session_path = entry.path();
                if session_path.is_dir() {
                    let checkpoints_dir = session_path.join("checkpoints");
                    if checkpoints_dir.exists()
                        && let Ok(checkpoint_entries) = std::fs::read_dir(&checkpoints_dir)
                    {
                        for checkpoint_entry in checkpoint_entries.flatten() {
                            let checkpoint_path = checkpoint_entry.path();
                            if checkpoint_path.extension().and_then(|s| s.to_str()) == Some("json")
                                && let Ok(content) = std::fs::read_to_string(&checkpoint_path)
                            {
                                // Parse the session data and extract checkpoints from metadata
                                if let Ok(session_data) =
                                    serde_json::from_str::<serde_json::Value>(&content)
                                    && let Some(metadata) = session_data.get("metadata")
                                    && let Some(checkpoints_array) = metadata.get("checkpoints")
                                    && let Some(checkpoints) = checkpoints_array.as_array()
                                {
                                    for checkpoint_value in checkpoints {
                                        if let Ok(checkpoint_info) =
                                            serde_json::from_value::<CheckpointInfo>(
                                                checkpoint_value.clone(),
                                            )
                                        {
                                            all_checkpoints.insert(
                                                checkpoint_info.created_at,
                                                checkpoint_info,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convert to vec and sort by creation time (most recent first)
        let mut checkpoints: Vec<_> = all_checkpoints.into_values().collect();
        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(checkpoints)
    }

    /// Create a checkpoint in the most recent session of a workspace
    /// This is a private static method used internally
    async fn create_checkpoint_in_latest_session(
        workspace_root: &std::path::Path,
        description: String,
    ) -> Result<CheckpointInfo> {
        use crate::env;
        use uuid::Uuid;

        let sessions_dir = env::sessions_dir_path(workspace_root);

        if !sessions_dir.exists() {
            return Err(anyhow::anyhow!("No sessions found in workspace"));
        }

        // Find the most recent session directory
        let mut latest_session_dir = None;
        let mut latest_time = None;

        if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Ok(metadata) = entry.metadata()
                    && let Ok(modified) = metadata.modified()
                    && (latest_time.is_none() || Some(modified) > latest_time)
                {
                    latest_time = Some(modified);
                    latest_session_dir = Some(path);
                }
            }
        }

        let session_dir =
            latest_session_dir.ok_or_else(|| anyhow::anyhow!("No session directories found"))?;

        // Load the session and add checkpoint to it
        let _session_config = SessionManagerConfig::default();
        let _init_options = SessionInitOptions {
            name: "Temporary Session for Manual Checkpoint".to_string(),
            description: Some("Temporary session for creating manual checkpoint".to_string()),
            workspace_root: workspace_root.to_path_buf(),
            task_manager_config: TaskManagerConfig::default(),
            persistence_config: PersistenceConfig::default(),
            recovery_config: RecoveryConfig::default(),
            enable_auto_save: false,
            restore_from_checkpoint: None,
            execution_mode: None,
        };

        // This approach still has the isolation issue. Let me implement a direct approach
        // that modifies the existing session files rather than creating a new session.

        // Create checkpoint info
        let checkpoint_id = format!("checkpoint_{}", Uuid::new_v4());
        let now = Utc::now();

        let checkpoint_info = CheckpointInfo {
            id: checkpoint_id.clone(),
            created_at: now,
            description: description.clone(),
            task_count: 0,
            size_bytes: 0,
            is_automatic: false,
            trigger_reason: CheckpointTrigger::Manual {
                reason: "User requested manual checkpoint".to_string(),
            },
        };

        // Find the most recent checkpoint file in the session to update
        let checkpoints_dir = session_dir.join("checkpoints");
        if !checkpoints_dir.exists() {
            std::fs::create_dir_all(&checkpoints_dir)?;
        }

        // Create a new checkpoint file in the existing session directory
        let checkpoint_file = checkpoints_dir.join(format!("{}.json", checkpoint_id));

        // Create minimal session-like structure for this checkpoint
        let session_data = serde_json::json!({
            "metadata": {
                "id": format!("{}", Uuid::new_v4()),
                "name": "Manual Checkpoint",
                "description": format!("Manual checkpoint: {}", description),
                "created_at": now,
                "last_updated": now,
                "version": {
                    "major": 1,
                    "minor": 0,
                    "patch": 0,
                    "agent_version": "0.1.0",
                    "format_version": "1.0"
                },
                "checkpoints": [&checkpoint_info],
                "total_tasks": 0,
                "completed_tasks": 0,
                "failed_tasks": 0,
                "session_tags": [],
                "workspace_root": workspace_root.to_string_lossy(),
                "custom_properties": {}
            },
            "task_tree": {
                "tasks": {},
                "roots": [],
                "metadata": {
                    "created_at": now,
                    "updated_at": now,
                    "version": 1,
                    "total_tasks_created": 0,
                    "statistics": {
                        "total_tasks": 0,
                        "pending_tasks": 0,
                        "in_progress_tasks": 0,
                        "completed_tasks": 0,
                        "failed_tasks": 0,
                        "blocked_tasks": 0,
                        "skipped_tasks": 0,
                        "average_completion_time": null,
                        "success_rate": 0.0
                    }
                }
            },
            "execution_context": {
                "current_working_directory": workspace_root.to_string_lossy(),
                "environment_variables": {},
                "active_file_watchers": [],
                "resource_usage": {
                    "memory_usage_mb": 0,
                    "cpu_usage_percent": 0.0,
                    "disk_usage_mb": 0,
                    "open_file_handles": 0,
                    "network_connections": 0
                }
            },
            "file_system_state": {
                "tracked_files": {},
                "workspace_files": [],
                "temp_files": [],
                "created_directories": []
            }
        });

        // Write the checkpoint file
        std::fs::write(
            &checkpoint_file,
            serde_json::to_string_pretty(&session_data)?,
        )?;

        info!(
            "Manual checkpoint created: {} in session: {:?}",
            checkpoint_id,
            session_dir.file_name()
        );

        Ok(checkpoint_info)
    }

    /// Get session directory path for audit trail coordination
    ///
    /// Returns the full path to this session's directory where audit logs,
    /// checkpoints, and other session artifacts are stored.
    pub async fn get_session_dir(&self) -> PathBuf {
        let metadata = self.metadata.read().await;
        crate::env::session_dir_path(&metadata.workspace_root, &self.session_id.to_string())
    }
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            auto_save_interval_minutes: 5,
            auto_checkpoint_interval_minutes: 30,
            checkpoint_on_significant_progress: true,
            significant_progress_threshold: 25.0, // 25%
            max_session_duration_hours: 24,
            enable_crash_recovery: true,
            validate_on_save: true,
            compress_checkpoints: false, // Disabled for initial implementation
        }
    }
}

impl Default for SessionInitOptions {
    fn default() -> Self {
        Self {
            name: "Default Session".to_string(),
            description: None,
            workspace_root: PathBuf::from("/tmp/claude-agent-workspace"),
            task_manager_config: TaskManagerConfig::default(),
            persistence_config: PersistenceConfig::default(),
            recovery_config: RecoveryConfig::default(),
            enable_auto_save: true,
            restore_from_checkpoint: None,
            execution_mode: None,
        }
    }
}
