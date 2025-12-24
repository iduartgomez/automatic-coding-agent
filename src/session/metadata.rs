use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Unique session identifier
pub type SessionId = uuid::Uuid;

/// Container information bound to a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContainerInfo {
    /// Container ID from Docker/Podman
    pub container_id: String,
    /// Container name (typically aca-{session_id})
    pub container_name: String,
    /// Container image used
    pub image: String,
    /// When the container was created
    pub created_at: DateTime<Utc>,
    /// Current container status
    pub status: ContainerStatus,
    /// Resource limits applied
    pub resource_limits: Option<ContainerResourceLimits>,
}

/// Container lifecycle status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerStatus {
    /// Container is running
    Running,
    /// Container is stopped
    Stopped,
    /// Container has been removed
    Removed,
    /// Container status is unknown
    Unknown,
}

/// Resource limits applied to a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResourceLimits {
    /// Memory limit in bytes
    pub memory_bytes: Option<i64>,
    /// CPU quota (microseconds per period)
    pub cpu_quota: Option<i64>,
}

impl SessionContainerInfo {
    /// Create new container info
    pub fn new(
        container_id: String,
        container_name: String,
        image: String,
        resource_limits: Option<ContainerResourceLimits>,
    ) -> Self {
        Self {
            container_id,
            container_name,
            image,
            created_at: Utc::now(),
            status: ContainerStatus::Running,
            resource_limits,
        }
    }

    /// Mark container as stopped
    pub fn mark_stopped(&mut self) {
        self.status = ContainerStatus::Stopped;
    }

    /// Mark container as removed
    pub fn mark_removed(&mut self) {
        self.status = ContainerStatus::Removed;
    }

    /// Check if container is running
    pub fn is_running(&self) -> bool {
        self.status == ContainerStatus::Running
    }
}

/// Session metadata and versioning information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: SessionId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub version: SessionVersion,
    pub checkpoints: Vec<CheckpointInfo>,
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub session_tags: Vec<String>,
    pub workspace_root: PathBuf,
    pub custom_properties: HashMap<String, serde_json::Value>,
    /// Execution mode used for this session (host or container)
    #[serde(default)]
    pub execution_mode: Option<crate::executor::RuntimeMode>,
    /// Container information if running in container mode
    #[serde(default)]
    pub container_info: Option<SessionContainerInfo>,
}

/// Session version information for compatibility tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub agent_version: String,
    pub format_version: String,
}

/// Information about a session checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub task_count: u32,
    pub size_bytes: u64,
    pub is_automatic: bool,
    pub trigger_reason: CheckpointTrigger,
}

/// Reasons for creating a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointTrigger {
    Manual { reason: String },
    Automatic { trigger: AutoTrigger },
    Error { error_type: String },
    Milestone { milestone: String },
}

/// Automatic checkpoint triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutoTrigger {
    TimeInterval { minutes: u32 },
    TaskCompletion { count: u32 },
    SignificantProgress { percentage: f32 },
    BeforeRiskyOperation,
}

/// Session statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatistics {
    pub total_execution_time: chrono::Duration,
    pub total_checkpoints: u32,
    pub data_size_mb: f64,
    pub average_checkpoint_size_mb: f64,
    pub recovery_count: u32,
    pub last_recovery_at: Option<DateTime<Utc>>,
    pub performance_metrics: PerformanceMetrics,
}

/// Performance metrics for session operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub average_save_time_ms: f64,
    pub average_load_time_ms: f64,
    pub largest_checkpoint_mb: f64,
    pub compression_ratio: f64,
    pub io_operations_count: u64,
}

impl SessionMetadata {
    /// Create new session metadata
    pub fn new(name: String, workspace_root: PathBuf) -> Self {
        Self {
            id: SessionId::new_v4(),
            name,
            description: None,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            version: SessionVersion::current(),
            checkpoints: Vec::new(),
            total_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            session_tags: Vec::new(),
            workspace_root,
            custom_properties: HashMap::new(),
            execution_mode: None,
            container_info: None,
        }
    }

    /// Set container information for this session
    pub fn set_container_info(&mut self, container_info: SessionContainerInfo) {
        self.container_info = Some(container_info);
        self.last_updated = Utc::now();
    }

    /// Clear container information (e.g., when container is removed)
    pub fn clear_container_info(&mut self) {
        if let Some(ref mut info) = self.container_info {
            info.mark_removed();
        }
        self.last_updated = Utc::now();
    }

    /// Get container ID if available
    pub fn container_id(&self) -> Option<&str> {
        self.container_info
            .as_ref()
            .map(|c| c.container_id.as_str())
    }

    /// Check if this session has an active container
    pub fn has_active_container(&self) -> bool {
        self.container_info.as_ref().is_some_and(|c| c.is_running())
    }

    /// Update session statistics
    pub fn update_statistics(&mut self, total_tasks: u32, completed_tasks: u32, failed_tasks: u32) {
        self.total_tasks = total_tasks;
        self.completed_tasks = completed_tasks;
        self.failed_tasks = failed_tasks;
        self.last_updated = Utc::now();
    }

    /// Add a checkpoint record
    pub fn add_checkpoint(&mut self, checkpoint: CheckpointInfo) {
        self.checkpoints.push(checkpoint);
        self.last_updated = Utc::now();
    }

    /// Get the latest checkpoint
    pub fn latest_checkpoint(&self) -> Option<&CheckpointInfo> {
        self.checkpoints.last()
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f32 {
        if self.total_tasks == 0 {
            0.0
        } else {
            (self.completed_tasks as f32 / self.total_tasks as f32) * 100.0
        }
    }

    /// Check if session is compatible with current version
    pub fn is_compatible(&self) -> bool {
        let current = SessionVersion::current();
        self.version.major == current.major && self.version.format_version == current.format_version
    }
}

impl SessionVersion {
    /// Get current session version
    pub fn current() -> Self {
        Self {
            major: 1,
            minor: 0,
            patch: 0,
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            format_version: "1.0".to_string(),
        }
    }

    /// Check compatibility with another version
    pub fn is_compatible_with(&self, other: &SessionVersion) -> bool {
        self.major == other.major && self.format_version == other.format_version
    }
}

impl Default for SessionStatistics {
    fn default() -> Self {
        Self {
            total_execution_time: chrono::Duration::zero(),
            total_checkpoints: 0,
            data_size_mb: 0.0,
            average_checkpoint_size_mb: 0.0,
            recovery_count: 0,
            last_recovery_at: None,
            performance_metrics: PerformanceMetrics::default(),
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            average_save_time_ms: 0.0,
            average_load_time_ms: 0.0,
            largest_checkpoint_mb: 0.0,
            compression_ratio: 1.0,
            io_operations_count: 0,
        }
    }
}
