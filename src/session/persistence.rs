use crate::env;
use crate::session::metadata::*;
use crate::task::tree::TaskTree;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

/// Complete session state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub metadata: SessionMetadata,
    pub task_tree: TaskTree,
    pub execution_context: ExecutionContext,
    pub file_system_state: FileSystemState,
}

/// Execution context state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub current_working_directory: PathBuf,
    pub environment_variables: std::collections::HashMap<String, String>,
    pub active_file_watchers: Vec<String>,
    pub resource_usage: ResourceUsageSnapshot,
}

/// File system state tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileSystemState {
    pub tracked_files: std::collections::HashMap<PathBuf, FileMetadata>,
    pub workspace_files: Vec<PathBuf>,
    pub temp_files: Vec<PathBuf>,
    pub created_directories: Vec<PathBuf>,
}

/// Metadata for tracked files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
    pub is_generated: bool,
}

/// Resource usage snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageSnapshot {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub disk_usage_mb: u64,
    pub open_file_handles: u32,
    pub network_connections: u32,
}

/// Atomic persistence manager with transaction support
pub struct PersistenceManager {
    workspace_root: PathBuf,
    session_id: String,
    session_dir: PathBuf,
    temp_dir: PathBuf,
    pub config: PersistenceConfig,
}

/// Configuration for persistence operations
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    pub enable_compression: bool,
    pub backup_count: u32,
    pub atomic_writes: bool,
    pub checksum_validation: bool,
    pub auto_cleanup: bool,
    pub max_checkpoint_age_hours: u32,
}

/// Result of a persistence operation
#[derive(Debug)]
pub struct PersistenceResult {
    pub success: bool,
    pub bytes_written: u64,
    pub duration_ms: u64,
    pub compression_ratio: Option<f64>,
    pub checksum: String,
}

/// Transaction for atomic operations
pub struct PersistenceTransaction {
    transaction_id: String,
    temp_files: Vec<PathBuf>,
    rollback_data: Vec<RollbackEntry>,
}

/// Rollback information for transaction recovery
#[derive(Debug)]
#[allow(dead_code)] // Transaction system is simplified in this implementation
struct RollbackEntry {
    file_path: PathBuf,
    operation: RollbackOperation,
    backup_path: Option<PathBuf>,
}

/// Types of operations that can be rolled back
#[derive(Debug)]
#[allow(dead_code)] // Transaction system is simplified in this implementation
enum RollbackOperation {
    Create,
    Modify { original_content: Vec<u8> },
    Delete { content: Vec<u8> },
}

impl PersistenceManager {
    /// Create a new persistence manager
    pub fn new(
        workspace_root: PathBuf,
        session_id: &str,
        config: PersistenceConfig,
    ) -> Result<Self> {
        // Create the .aca directory structure
        let aca_root = env::aca_dir_path(&workspace_root);
        let session_dir = env::session_dir_path(&workspace_root, session_id);

        // Create directory structure as per design specification
        let meta_dir = session_dir.join(env::session::META_DIR_NAME);
        let state_dir = session_dir.join("state");
        let claude_dir = session_dir.join("claude");
        let logs_dir = session_dir.join(env::session::LOGS_DIR_NAME);
        let checkpoints_dir = session_dir.join(env::session::CHECKPOINTS_DIR_NAME);
        let temp_dir = session_dir.join(env::session::TEMP_DIR_NAME);

        // Create conversation and other subdirectories
        let conversation_dir = claude_dir.join("conversation");
        let context_windows_dir = claude_dir.join("context_windows");
        let rate_limit_dir = claude_dir.join("rate_limit");
        let execution_logs_dir = logs_dir.join("execution");
        let claude_interactions_dir = logs_dir.join(env::session::CLAUDE_INTERACTIONS_DIR_NAME);
        let errors_dir = logs_dir.join(env::session::ERRORS_DIR_NAME);

        // Ensure all directories exist
        for dir in [
            &aca_root,
            &session_dir,
            &meta_dir,
            &state_dir,
            &claude_dir,
            &logs_dir,
            &checkpoints_dir,
            &temp_dir,
            &conversation_dir,
            &context_windows_dir,
            &rate_limit_dir,
            &execution_logs_dir,
            &claude_interactions_dir,
            &errors_dir,
        ] {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }

        Ok(Self {
            workspace_root,
            session_id: session_id.to_string(),
            session_dir,
            temp_dir,
            config,
        })
    }

    /// Save session state atomically
    pub async fn save_session_state(&self, state: &SessionState) -> Result<PersistenceResult> {
        let _start_time = std::time::Instant::now();

        // Create transaction for atomic operation
        let transaction = self.begin_transaction().await?;

        let result = if self.config.atomic_writes {
            self.save_with_transaction(state, &transaction).await
        } else {
            self.save_direct(state).await
        };

        match result {
            Ok(persistence_result) => {
                self.commit_transaction(transaction).await?;
                info!(
                    "Session saved successfully: {} bytes in {}ms",
                    persistence_result.bytes_written, persistence_result.duration_ms
                );
                Ok(persistence_result)
            }
            Err(e) => {
                self.rollback_transaction(transaction).await?;
                error!("Session save failed: {}", e);
                Err(e)
            }
        }
    }

    /// Load session state with validation
    pub async fn load_session_state(&self) -> Result<SessionState> {
        let session_file = env::session_state_file_path(&self.workspace_root, &self.session_id);

        if !session_file.exists() {
            return Err(anyhow::anyhow!("Session file not found"));
        }

        let start_time = std::time::Instant::now();

        // Read and deserialize
        let content = async_fs::read(&session_file)
            .await
            .context("Failed to read session file")?;

        let decompressed_content = if self.config.enable_compression {
            self.decompress_data(&content)?
        } else {
            content
        };

        // Validate checksum if enabled
        if self.config.checksum_validation {
            self.validate_checksum(&decompressed_content, &session_file)
                .await?;
        }

        let state: SessionState = serde_json::from_slice(&decompressed_content)
            .context("Failed to deserialize session state")?;

        // Validate session compatibility
        if !state.metadata.is_compatible() {
            warn!(
                "Session version may be incompatible: {:?}",
                state.metadata.version
            );
        }

        let duration = start_time.elapsed();
        info!("Session loaded successfully in {}ms", duration.as_millis());

        Ok(state)
    }

    /// Create a checkpoint with the current state
    pub async fn create_checkpoint(
        &self,
        state: &SessionState,
        description: String,
        trigger: CheckpointTrigger,
    ) -> Result<CheckpointInfo> {
        let checkpoint_uuid = uuid::Uuid::new_v4().to_string();
        let checkpoint_id = format!("checkpoint_{}", checkpoint_uuid);
        let checkpoint_file =
            env::checkpoint_file_path(&self.workspace_root, &self.session_id, &checkpoint_id);

        let _start_time = std::time::Instant::now();

        // Save checkpoint
        let persistence_result = self.save_to_file(state, &checkpoint_file).await?;

        let checkpoint_info = CheckpointInfo {
            id: checkpoint_id,
            created_at: chrono::Utc::now(),
            description,
            task_count: state.task_tree.tasks.len() as u32,
            size_bytes: persistence_result.bytes_written,
            is_automatic: matches!(trigger, CheckpointTrigger::Automatic { .. }),
            trigger_reason: trigger,
        };

        info!(
            "Checkpoint created: {} ({} bytes)",
            checkpoint_info.id, checkpoint_info.size_bytes
        );

        Ok(checkpoint_info)
    }

    /// Restore from a specific checkpoint
    pub async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<SessionState> {
        let checkpoint_file =
            env::checkpoint_file_path(&self.workspace_root, &self.session_id, checkpoint_id);

        if !checkpoint_file.exists() {
            return Err(anyhow::anyhow!("Checkpoint {} not found", checkpoint_id));
        }

        info!("Restoring from checkpoint: {}", checkpoint_id);

        // Load checkpoint data
        let content = async_fs::read(&checkpoint_file)
            .await
            .context("Failed to read checkpoint file")?;

        let decompressed_content = if self.config.enable_compression {
            self.decompress_data(&content)?
        } else {
            content
        };

        let state: SessionState = serde_json::from_slice(&decompressed_content)
            .context("Failed to deserialize checkpoint state")?;

        info!("Successfully restored from checkpoint: {}", checkpoint_id);
        Ok(state)
    }

    /// List available checkpoints
    pub async fn list_checkpoints(&self) -> Result<Vec<String>> {
        let mut checkpoints = Vec::new();

        let checkpoints_dir =
            env::session_checkpoints_dir_path(&self.workspace_root, &self.session_id);
        if !checkpoints_dir.exists() {
            return Ok(checkpoints);
        }

        let mut entries = async_fs::read_dir(&checkpoints_dir)
            .await
            .context("Failed to read checkpoints directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name()
                && let Some(name_str) = file_name.to_str()
                && name_str.starts_with("checkpoint_")
                && name_str.ends_with(".json")
            {
                let checkpoint_id = name_str.strip_suffix(".json").unwrap();
                checkpoints.push(checkpoint_id.to_string());
            }
        }

        checkpoints.sort();
        Ok(checkpoints)
    }

    /// Clean up old checkpoints based on configuration
    pub async fn cleanup_old_checkpoints(&self) -> Result<u32> {
        if !self.config.auto_cleanup {
            return Ok(0);
        }

        let cutoff_time = chrono::Utc::now()
            - chrono::Duration::hours(self.config.max_checkpoint_age_hours as i64);

        let checkpoints = self.list_checkpoints().await?;
        let mut cleaned_count = 0;

        for checkpoint_id in checkpoints {
            let checkpoint_file = self
                .session_dir
                .join(format!("checkpoint_{}.json", checkpoint_id));

            if let Ok(metadata) = async_fs::metadata(&checkpoint_file).await
                && let Ok(modified) = metadata.modified()
            {
                let modified_utc: chrono::DateTime<chrono::Utc> = modified.into();

                if modified_utc < cutoff_time {
                    if let Err(e) = async_fs::remove_file(&checkpoint_file).await {
                        warn!("Failed to remove old checkpoint {}: {}", checkpoint_id, e);
                    } else {
                        debug!("Removed old checkpoint: {}", checkpoint_id);
                        cleaned_count += 1;
                    }
                }
            }
        }

        if cleaned_count > 0 {
            info!("Cleaned up {} old checkpoints", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Begin atomic transaction
    async fn begin_transaction(&self) -> Result<PersistenceTransaction> {
        let transaction_id = uuid::Uuid::new_v4().to_string();

        Ok(PersistenceTransaction {
            transaction_id,
            temp_files: Vec::new(),
            rollback_data: Vec::new(),
        })
    }

    /// Commit transaction by moving temp files to final locations
    async fn commit_transaction(&self, transaction: PersistenceTransaction) -> Result<()> {
        // Move temp files to final locations
        for temp_file in transaction.temp_files {
            if let Some(final_path) = self.get_final_path_for_temp(&temp_file) {
                async_fs::rename(&temp_file, &final_path)
                    .await
                    .context("Failed to commit transaction file")?;
            }
        }

        debug!("Transaction committed: {}", transaction.transaction_id);
        Ok(())
    }

    /// Rollback transaction by restoring original state
    async fn rollback_transaction(&self, transaction: PersistenceTransaction) -> Result<()> {
        // Remove temp files
        for temp_file in transaction.temp_files {
            if temp_file.exists() {
                let _ = async_fs::remove_file(&temp_file).await;
            }
        }

        // Restore from rollback data
        for rollback_entry in transaction.rollback_data {
            match rollback_entry.operation {
                RollbackOperation::Create => {
                    // Remove created file
                    if rollback_entry.file_path.exists() {
                        let _ = async_fs::remove_file(&rollback_entry.file_path).await;
                    }
                }
                RollbackOperation::Modify { original_content } => {
                    // Restore original content
                    let _ = async_fs::write(&rollback_entry.file_path, original_content).await;
                }
                RollbackOperation::Delete { content } => {
                    // Recreate deleted file
                    let _ = async_fs::write(&rollback_entry.file_path, content).await;
                }
            }
        }

        warn!("Transaction rolled back: {}", transaction.transaction_id);
        Ok(())
    }

    /// Save with transaction support
    async fn save_with_transaction(
        &self,
        state: &SessionState,
        transaction: &PersistenceTransaction,
    ) -> Result<PersistenceResult> {
        let temp_file = self
            .temp_dir
            .join(format!("session_{}.json", transaction.transaction_id));
        let result = self.save_to_file(state, &temp_file).await?;

        // In this simplified implementation, we'll just copy to the final location
        let final_file = env::session_state_file_path(&self.workspace_root, &self.session_id);
        async_fs::copy(&temp_file, &final_file)
            .await
            .context("Failed to copy temp file to final location")?;

        Ok(result)
    }

    /// Save directly without transaction
    async fn save_direct(&self, state: &SessionState) -> Result<PersistenceResult> {
        let session_file = env::session_state_file_path(&self.workspace_root, &self.session_id);
        self.save_to_file(state, &session_file).await
    }

    /// Save state to a specific file
    async fn save_to_file(
        &self,
        state: &SessionState,
        file_path: &Path,
    ) -> Result<PersistenceResult> {
        let start_time = std::time::Instant::now();

        // Serialize the state
        let serialized =
            serde_json::to_vec_pretty(state).context("Failed to serialize session state")?;

        // Compress if enabled
        let (final_data, compression_ratio) = if self.config.enable_compression {
            let compressed = self.compress_data(&serialized)?;
            let ratio = serialized.len() as f64 / compressed.len() as f64;
            (compressed, Some(ratio))
        } else {
            (serialized, None)
        };

        // Calculate checksum
        let checksum = self.calculate_checksum(&final_data);

        // Write to file
        let mut file = async_fs::File::create(file_path)
            .await
            .context("Failed to create session file")?;

        file.write_all(&final_data)
            .await
            .context("Failed to write session data")?;

        file.sync_all()
            .await
            .context("Failed to sync session file")?;

        // Write checksum file if validation enabled
        if self.config.checksum_validation {
            let checksum_file = file_path.with_extension("checksum");
            async_fs::write(&checksum_file, &checksum)
                .await
                .context("Failed to write checksum file")?;
        }

        let duration = start_time.elapsed();

        Ok(PersistenceResult {
            success: true,
            bytes_written: final_data.len() as u64,
            duration_ms: duration.as_millis() as u64,
            compression_ratio,
            checksum,
        })
    }

    /// Get final path for a temp file (simplified implementation)
    fn get_final_path_for_temp(&self, temp_path: &Path) -> Option<PathBuf> {
        if let Some(file_name) = temp_path.file_name()
            && let Some(name_str) = file_name.to_str()
            && name_str.starts_with("session_")
        {
            return Some(env::session_state_file_path(
                &self.workspace_root,
                &self.session_id,
            ));
        }
        None
    }

    /// Compress data (placeholder implementation)
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // In a real implementation, would use a compression library like flate2
        // For now, return original data
        Ok(data.to_vec())
    }

    /// Decompress data (placeholder implementation)
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // In a real implementation, would decompress the data
        // For now, return original data
        Ok(data.to_vec())
    }

    /// Calculate checksum for data
    fn calculate_checksum(&self, data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Validate checksum of loaded data
    async fn validate_checksum(&self, data: &[u8], file_path: &Path) -> Result<()> {
        let checksum_file = file_path.with_extension("checksum");

        if checksum_file.exists() {
            let stored_checksum = async_fs::read_to_string(&checksum_file)
                .await
                .context("Failed to read checksum file")?;

            let calculated_checksum = self.calculate_checksum(data);

            if stored_checksum.trim() != calculated_checksum {
                return Err(anyhow::anyhow!("Checksum validation failed"));
            }
        }

        Ok(())
    }
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enable_compression: false, // Disable for simplicity in initial implementation
            backup_count: 5,
            atomic_writes: true,
            checksum_validation: true,
            auto_cleanup: true,
            max_checkpoint_age_hours: 168, // 1 week
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            current_working_directory: std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("/")),
            environment_variables: std::collections::HashMap::new(),
            active_file_watchers: Vec::new(),
            resource_usage: ResourceUsageSnapshot::default(),
        }
    }
}

impl Default for ResourceUsageSnapshot {
    fn default() -> Self {
        Self {
            memory_usage_mb: 0,
            cpu_usage_percent: 0.0,
            disk_usage_mb: 0,
            open_file_handles: 0,
            network_connections: 0,
        }
    }
}
