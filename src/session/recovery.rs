use crate::session::metadata::*;
use crate::session::persistence::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Recovery manager for session restoration and error recovery
pub struct RecoveryManager {
    persistence: PersistenceManager,
    pub config: RecoveryConfig,
}

/// Configuration for recovery operations
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub auto_recovery_enabled: bool,
    pub max_recovery_attempts: u32,
    pub recovery_timeout_minutes: u32,
    pub validate_state_on_recovery: bool,
    pub create_recovery_checkpoint: bool,
    pub preserve_corrupted_data: bool,
}

/// Information about a recovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryInfo {
    pub recovery_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub recovery_type: RecoveryType,
    pub source_checkpoint: Option<String>,
    pub recovered_tasks: u32,
    pub validation_errors: Vec<String>,
    pub success: bool,
}

/// Types of recovery operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryType {
    AutomaticCrashRecovery,
    ManualCheckpointRestore { checkpoint_id: String },
    CorruptionRecovery,
    PartialStateRecovery,
    EmergencyRecovery,
}

/// Result of a recovery operation
#[derive(Debug)]
pub struct RecoveryResult {
    pub success: bool,
    pub recovered_state: Option<SessionState>,
    pub recovery_info: RecoveryInfo,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// State validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub correctable_issues: Vec<CorrectableIssue>,
}

/// Validation error types
#[derive(Debug, Clone)]
pub enum ValidationError {
    CorruptedTaskTree { details: String },
    InvalidTaskReferences { task_ids: Vec<String> },
    MissingDependencies { dependencies: Vec<String> },
    CircularDependencies { cycle: Vec<String> },
    InconsistentMetadata { field: String, issue: String },
    FileSystemMismatch { expected: PathBuf, actual: PathBuf },
}

/// Issues that can be automatically corrected
#[derive(Debug, Clone)]
pub enum CorrectableIssue {
    OrphanedTasks { task_ids: Vec<String> },
    DuplicateTaskIds { duplicates: Vec<String> },
    OutdatedTimestamps { tasks: Vec<String> },
    MissingTaskMetadata { tasks: Vec<String> },
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(persistence: PersistenceManager, config: RecoveryConfig) -> Self {
        Self {
            persistence,
            config,
        }
    }

    /// Attempt automatic recovery from the most recent valid state
    pub async fn auto_recover(&self) -> Result<RecoveryResult> {
        info!("Starting automatic recovery");

        let recovery_id = uuid::Uuid::new_v4().to_string();
        let mut recovery_info = RecoveryInfo {
            recovery_id: recovery_id.clone(),
            started_at: Utc::now(),
            completed_at: None,
            recovery_type: RecoveryType::AutomaticCrashRecovery,
            source_checkpoint: None,
            recovered_tasks: 0,
            validation_errors: Vec::new(),
            success: false,
        };

        // Try to load the main session state first
        match self.persistence.load_session_state().await {
            Ok(state) => {
                info!("Main session state loaded successfully");

                if self.config.validate_state_on_recovery {
                    let validation = self.validate_session_state(&state).await?;
                    if validation.is_valid {
                        recovery_info.success = true;
                        recovery_info.completed_at = Some(Utc::now());
                        recovery_info.recovered_tasks = state.task_tree.tasks.len() as u32;

                        return Ok(RecoveryResult {
                            success: true,
                            recovered_state: Some(state),
                            recovery_info,
                            warnings: validation.warnings,
                            errors: Vec::new(),
                        });
                    } else {
                        warn!("Main session state validation failed, trying checkpoints");
                        recovery_info.validation_errors = validation
                            .errors
                            .iter()
                            .map(|e| format!("{:?}", e))
                            .collect();
                    }
                }
            }
            Err(e) => {
                warn!("Failed to load main session state: {}", e);
            }
        }

        // Try to recover from the most recent checkpoint
        if let Ok(checkpoints) = self.persistence.list_checkpoints().await {
            for checkpoint_id in checkpoints.iter().rev() {
                info!("Attempting recovery from checkpoint: {}", checkpoint_id);

                match self
                    .persistence
                    .restore_from_checkpoint(checkpoint_id)
                    .await
                {
                    Ok(state) => {
                        if self.config.validate_state_on_recovery {
                            let validation = self.validate_session_state(&state).await?;
                            if validation.is_valid {
                                recovery_info.success = true;
                                recovery_info.completed_at = Some(Utc::now());
                                recovery_info.source_checkpoint = Some(checkpoint_id.clone());
                                recovery_info.recovered_tasks = state.task_tree.tasks.len() as u32;

                                info!("Successfully recovered from checkpoint: {}", checkpoint_id);

                                return Ok(RecoveryResult {
                                    success: true,
                                    recovered_state: Some(state),
                                    recovery_info,
                                    warnings: validation.warnings,
                                    errors: Vec::new(),
                                });
                            } else {
                                warn!("Checkpoint {} validation failed", checkpoint_id);
                            }
                        } else {
                            // Accept without validation
                            recovery_info.success = true;
                            recovery_info.completed_at = Some(Utc::now());
                            recovery_info.source_checkpoint = Some(checkpoint_id.clone());
                            recovery_info.recovered_tasks = state.task_tree.tasks.len() as u32;

                            info!(
                                "Recovered from checkpoint: {} (no validation)",
                                checkpoint_id
                            );

                            return Ok(RecoveryResult {
                                success: true,
                                recovered_state: Some(state),
                                recovery_info,
                                warnings: vec!["State not validated".to_string()],
                                errors: Vec::new(),
                            });
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load checkpoint {}: {}", checkpoint_id, e);
                    }
                }
            }
        }

        // No successful recovery
        recovery_info.completed_at = Some(Utc::now());
        error!("All recovery attempts failed");

        Ok(RecoveryResult {
            success: false,
            recovered_state: None,
            recovery_info,
            warnings: Vec::new(),
            errors: vec!["All recovery attempts failed".to_string()],
        })
    }

    /// Recover from a specific checkpoint
    pub async fn recover_from_checkpoint(&self, checkpoint_id: &str) -> Result<RecoveryResult> {
        info!("Starting recovery from checkpoint: {}", checkpoint_id);

        let recovery_id = uuid::Uuid::new_v4().to_string();
        let mut recovery_info = RecoveryInfo {
            recovery_id: recovery_id.clone(),
            started_at: Utc::now(),
            completed_at: None,
            recovery_type: RecoveryType::ManualCheckpointRestore {
                checkpoint_id: checkpoint_id.to_string(),
            },
            source_checkpoint: Some(checkpoint_id.to_string()),
            recovered_tasks: 0,
            validation_errors: Vec::new(),
            success: false,
        };

        match self
            .persistence
            .restore_from_checkpoint(checkpoint_id)
            .await
        {
            Ok(state) => {
                let mut warnings = Vec::new();
                let mut errors = Vec::new();

                // Validate if configured
                if self.config.validate_state_on_recovery {
                    let validation = self.validate_session_state(&state).await?;
                    warnings.extend(validation.warnings);

                    if !validation.is_valid {
                        errors.extend(validation.errors.iter().map(|e| format!("{:?}", e)));
                        recovery_info.validation_errors = errors.clone();

                        // Still consider it a successful recovery but with warnings
                        if validation.correctable_issues.is_empty() {
                            warn!("Checkpoint has validation errors but recovered anyway");
                        }
                    }
                }

                recovery_info.success = true;
                recovery_info.completed_at = Some(Utc::now());
                recovery_info.recovered_tasks = state.task_tree.tasks.len() as u32;

                info!("Successfully recovered from checkpoint: {}", checkpoint_id);

                Ok(RecoveryResult {
                    success: true,
                    recovered_state: Some(state),
                    recovery_info,
                    warnings,
                    errors,
                })
            }
            Err(e) => {
                error!("Failed to recover from checkpoint {}: {}", checkpoint_id, e);

                recovery_info.completed_at = Some(Utc::now());

                Ok(RecoveryResult {
                    success: false,
                    recovered_state: None,
                    recovery_info,
                    warnings: Vec::new(),
                    errors: vec![e.to_string()],
                })
            }
        }
    }

    /// Validate session state integrity
    pub async fn validate_session_state(&self, state: &SessionState) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut correctable_issues = Vec::new();

        // Validate task tree integrity
        let task_tree = &state.task_tree;

        // Check for orphaned tasks
        let mut orphaned_tasks = Vec::new();
        for (&task_id, task) in &task_tree.tasks {
            if let Some(parent_id) = task.parent_id {
                if !task_tree.tasks.contains_key(&parent_id) {
                    orphaned_tasks.push(task_id.to_string());
                }
            }
        }

        if !orphaned_tasks.is_empty() {
            correctable_issues.push(CorrectableIssue::OrphanedTasks {
                task_ids: orphaned_tasks,
            });
        }

        // Check for circular dependencies
        for &task_id in task_tree.tasks.keys() {
            if task_tree.has_circular_dependency(task_id)? {
                errors.push(ValidationError::CircularDependencies {
                    cycle: vec![task_id.to_string()], // Simplified
                });
            }
        }

        // Check for broken child references
        for (&_task_id, task) in &task_tree.tasks {
            for &child_id in &task.children {
                if !task_tree.tasks.contains_key(&child_id) {
                    errors.push(ValidationError::InvalidTaskReferences {
                        task_ids: vec![child_id.to_string()],
                    });
                }
            }
        }

        // Check metadata consistency
        if !state.metadata.is_compatible() {
            warnings.push("Session version may be incompatible".to_string());
        }

        // Check file system state
        for file_path in state.file_system_state.tracked_files.keys() {
            if !file_path.exists() {
                warnings.push(format!("Tracked file missing: {}", file_path.display()));
            }
        }

        let is_valid = errors.is_empty();

        Ok(ValidationResult {
            is_valid,
            errors,
            warnings,
            correctable_issues,
        })
    }

    /// Attempt to correct issues automatically
    pub async fn auto_correct_issues(
        &self,
        mut state: SessionState,
        issues: Vec<CorrectableIssue>,
    ) -> Result<SessionState> {
        for issue in issues {
            match issue {
                CorrectableIssue::OrphanedTasks { task_ids } => {
                    info!("Correcting orphaned tasks: {:?}", task_ids);
                    // Remove orphaned tasks or attach them to root
                    for task_id_str in task_ids {
                        if let Ok(task_id) = task_id_str.parse() {
                            if let Some(task) = state.task_tree.tasks.get_mut(&task_id) {
                                task.parent_id = None; // Make it a root task
                            }
                        }
                    }
                }
                CorrectableIssue::DuplicateTaskIds { duplicates } => {
                    warn!(
                        "Found duplicate task IDs (cannot auto-correct): {:?}",
                        duplicates
                    );
                    // This would require more complex logic to merge or remove duplicates
                }
                CorrectableIssue::OutdatedTimestamps { tasks } => {
                    info!("Updating outdated timestamps for {} tasks", tasks.len());
                    for task_id_str in tasks {
                        if let Ok(task_id) = task_id_str.parse() {
                            if let Some(task) = state.task_tree.tasks.get_mut(&task_id) {
                                task.updated_at = Utc::now();
                            }
                        }
                    }
                }
                CorrectableIssue::MissingTaskMetadata { tasks } => {
                    info!("Adding missing metadata for {} tasks", tasks.len());
                    // Would add default metadata for tasks missing it
                }
            }
        }

        Ok(state)
    }

    /// Create an emergency recovery checkpoint before risky operations
    pub async fn create_emergency_checkpoint(&self, state: &SessionState) -> Result<String> {
        let checkpoint_info = self
            .persistence
            .create_checkpoint(
                state,
                "Emergency checkpoint before recovery".to_string(),
                CheckpointTrigger::Error {
                    error_type: "pre_recovery".to_string(),
                },
            )
            .await?;

        info!("Created emergency checkpoint: {}", checkpoint_info.id);
        Ok(checkpoint_info.id)
    }

    /// Get recovery statistics and history
    pub fn get_recovery_history(&self) -> Vec<RecoveryInfo> {
        // In a real implementation, this would load from persistent storage
        // For now, return empty vector
        Vec::new()
    }

    /// Check if automatic recovery should be attempted
    pub fn should_auto_recover(&self) -> bool {
        self.config.auto_recovery_enabled
    }
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            auto_recovery_enabled: true,
            max_recovery_attempts: 3,
            recovery_timeout_minutes: 30,
            validate_state_on_recovery: true,
            create_recovery_checkpoint: true,
            preserve_corrupted_data: true,
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::CorruptedTaskTree { details } => {
                write!(f, "Corrupted task tree: {}", details)
            }
            ValidationError::InvalidTaskReferences { task_ids } => {
                write!(f, "Invalid task references: {:?}", task_ids)
            }
            ValidationError::MissingDependencies { dependencies } => {
                write!(f, "Missing dependencies: {:?}", dependencies)
            }
            ValidationError::CircularDependencies { cycle } => {
                write!(f, "Circular dependencies detected: {:?}", cycle)
            }
            ValidationError::InconsistentMetadata { field, issue } => {
                write!(f, "Inconsistent metadata in {}: {}", field, issue)
            }
            ValidationError::FileSystemMismatch { expected, actual } => {
                write!(
                    f,
                    "File system mismatch - expected: {}, actual: {}",
                    expected.display(),
                    actual.display()
                )
            }
        }
    }
}
