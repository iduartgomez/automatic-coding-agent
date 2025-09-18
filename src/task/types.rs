use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for tasks
pub type TaskId = Uuid;

/// Unique identifier for Claude Code sessions
pub type ClaudeSessionId = Uuid;

/// Core task structure with comprehensive metadata and state tracking
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub parent_id: Option<TaskId>,
    pub children: Vec<TaskId>,
    pub dependencies: Vec<TaskId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: TaskMetadata,
    pub execution_history: Vec<ExecutionRecord>,
}

/// Task status with detailed state information
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskStatus {
    /// Task is ready to be executed but hasn't started
    Pending,
    /// Task is currently being processed
    InProgress {
        started_at: DateTime<Utc>,
        estimated_completion: Option<DateTime<Utc>>,
    },
    /// Task is temporarily blocked by external factors
    Blocked {
        reason: String,
        blocked_at: DateTime<Utc>,
        retry_after: Option<DateTime<Utc>>,
    },
    /// Task completed successfully
    Completed {
        completed_at: DateTime<Utc>,
        result: TaskResult,
    },
    /// Task failed and cannot be recovered automatically
    Failed {
        failed_at: DateTime<Utc>,
        error: TaskError,
        retry_count: u32,
    },
    /// Task was deliberately skipped
    Skipped {
        reason: String,
        skipped_at: DateTime<Utc>,
    },
}

/// Rich metadata for task management and scheduling
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskMetadata {
    pub priority: TaskPriority,
    pub estimated_complexity: Option<ComplexityLevel>,
    pub estimated_duration: Option<Duration>,
    pub repository_refs: Vec<RepositoryRef>,
    pub file_refs: Vec<FileRef>,
    pub tags: Vec<String>,
    pub context_requirements: ContextRequirements,
}

/// Task priority levels with numeric values for scoring
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 10,
    High = 8,
    Normal = 5,
    Low = 3,
    Background = 1,
}

/// Complexity estimation for resource planning
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComplexityLevel {
    Trivial,    // < 5 minutes
    Simple,     // 5-15 minutes
    Moderate,   // 15-60 minutes
    Complex,    // 1-4 hours
    Epic,       // > 4 hours
}

/// Repository reference for VCS operations
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepositoryRef {
    pub name: String,
    pub url: String,
    pub branch: Option<String>,
    pub commit: Option<String>,
}

/// File reference for context and change tracking
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileRef {
    pub path: PathBuf,
    pub repository: String,
    pub line_range: Option<(u32, u32)>,
    pub importance: FileImportance,
}

/// Importance level for file references
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileImportance {
    Critical,  // Core implementation files
    High,      // Important supporting files
    Medium,    // Related files
    Low,       // Reference files
}

/// Context requirements for task execution
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContextRequirements {
    pub required_files: Vec<PathBuf>,
    pub required_repositories: Vec<String>,
    pub build_dependencies: Vec<String>,
    pub environment_vars: HashMap<String, String>,
    pub claude_context_keys: Vec<String>,
}

/// Task dependency relationship
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskDependency {
    pub task_id: TaskId,
    pub dependency_type: DependencyType,
    pub required_status: Vec<TaskStatus>,
}

/// Types of task dependencies
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DependencyType {
    /// Must complete before this task can start
    Prerequisite,
    /// Should complete before this task starts (soft dependency)
    Preferred,
    /// Must be running concurrently
    Concurrent,
    /// Must not run at the same time
    Exclusive,
}

/// Execution record for task history tracking
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecutionRecord {
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: TaskStatus,
    pub claude_session_id: Option<ClaudeSessionId>,
    pub resources_used: ResourceUsage,
    pub files_modified: Vec<PathBuf>,
    pub errors: Vec<String>,
}

/// Resource usage tracking
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResourceUsage {
    pub max_memory_mb: u64,
    pub cpu_time_seconds: f64,
    pub disk_io_mb: u64,
    pub network_requests: u32,
}

/// Task execution result
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskResult {
    Success {
        output: serde_json::Value,
        files_created: Vec<PathBuf>,
        files_modified: Vec<PathBuf>,
        build_artifacts: Vec<PathBuf>,
    },
    Partial {
        completed_work: serde_json::Value,
        remaining_work: Vec<TaskSpec>,
        files_modified: Vec<PathBuf>,
    },
}

/// Task execution errors
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskError {
    /// Claude Code API errors
    ClaudeError {
        message: String,
        error_code: Option<String>,
        retry_possible: bool,
    },
    /// Build or compilation errors
    BuildError {
        exit_code: i32,
        stdout: String,
        stderr: String,
        affected_files: Vec<PathBuf>,
    },
    /// File system errors
    FileSystemError {
        message: String,
        path: Option<PathBuf>,
        operation: String,
    },
    /// Resource exhaustion
    ResourceError {
        resource_type: String,
        limit_exceeded: String,
        current_usage: String,
    },
    /// Dependency errors
    DependencyError {
        message: String,
        missing_dependencies: Vec<String>,
        conflict_dependencies: Vec<String>,
    },
    /// Timeout errors
    TimeoutError {
        operation: String,
        timeout_duration: Duration,
        elapsed_time: Duration,
    },
    /// Generic errors
    Other {
        message: String,
        source: Option<String>,
    },
}

/// Task specification for creating new tasks
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskSpec {
    pub title: String,
    pub description: String,
    pub metadata: TaskMetadata,
    pub dependencies: Vec<TaskId>,
}

impl Task {
    /// Create a new task with the given specification
    pub fn new(spec: TaskSpec, parent_id: Option<TaskId>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: spec.title,
            description: spec.description,
            status: TaskStatus::Pending,
            parent_id,
            children: Vec::new(),
            dependencies: spec.dependencies,
            created_at: now,
            updated_at: now,
            metadata: spec.metadata,
            execution_history: Vec::new(),
        }
    }

    /// Check if task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed { .. } | TaskStatus::Failed { .. } | TaskStatus::Skipped { .. }
        )
    }

    /// Check if task is currently runnable
    pub fn is_runnable(&self) -> bool {
        matches!(self.status, TaskStatus::Pending)
    }

    /// Check if task is currently running
    pub fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::InProgress { .. })
    }

    /// Check if task is blocked
    pub fn is_blocked(&self) -> bool {
        matches!(self.status, TaskStatus::Blocked { .. })
    }

    /// Get task age since creation
    pub fn age(&self) -> Duration {
        Utc::now().signed_duration_since(self.created_at)
    }

    /// Get task runtime if currently running
    pub fn runtime(&self) -> Option<Duration> {
        if let TaskStatus::InProgress { started_at, .. } = self.status {
            Some(Utc::now().signed_duration_since(started_at))
        } else {
            None
        }
    }

    /// Update task status and timestamp
    pub fn update_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Add execution record
    pub fn add_execution_record(&mut self, record: ExecutionRecord) {
        self.execution_history.push(record);
        self.updated_at = Utc::now();
    }

    /// Get priority as numeric value for scoring
    pub fn priority_value(&self) -> u8 {
        self.metadata.priority.clone() as u8
    }
}

impl ContextRequirements {
    /// Create empty context requirements
    pub fn new() -> Self {
        Self {
            required_files: Vec::new(),
            required_repositories: Vec::new(),
            build_dependencies: Vec::new(),
            environment_vars: HashMap::new(),
            claude_context_keys: Vec::new(),
        }
    }

    /// Merge with another context requirements
    pub fn merge_with(&mut self, other: &ContextRequirements) {
        self.required_files.extend(other.required_files.iter().cloned());
        self.required_repositories.extend(other.required_repositories.iter().cloned());
        self.build_dependencies.extend(other.build_dependencies.iter().cloned());
        self.environment_vars.extend(other.environment_vars.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.claude_context_keys.extend(other.claude_context_keys.iter().cloned());

        // Remove duplicates
        self.required_files.sort();
        self.required_files.dedup();
        self.required_repositories.sort();
        self.required_repositories.dedup();
        self.build_dependencies.sort();
        self.build_dependencies.dedup();
        self.claude_context_keys.sort();
        self.claude_context_keys.dedup();
    }

    /// Check if context is empty
    pub fn is_empty(&self) -> bool {
        self.required_files.is_empty()
            && self.required_repositories.is_empty()
            && self.build_dependencies.is_empty()
            && self.environment_vars.is_empty()
            && self.claude_context_keys.is_empty()
    }
}

impl Default for ContextRequirements {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskPriority {
    /// Get numeric value for calculations
    pub fn value(&self) -> u8 {
        self.clone() as u8
    }
}

impl ComplexityLevel {
    /// Get estimated duration for this complexity level
    pub fn estimated_duration(&self) -> Duration {
        match self {
            ComplexityLevel::Trivial => Duration::minutes(5),
            ComplexityLevel::Simple => Duration::minutes(15),
            ComplexityLevel::Moderate => Duration::minutes(60),
            ComplexityLevel::Complex => Duration::hours(4),
            ComplexityLevel::Epic => Duration::hours(8),
        }
    }

    /// Get numeric value for calculations (0-4)
    pub fn value(&self) -> u8 {
        match self {
            ComplexityLevel::Trivial => 0,
            ComplexityLevel::Simple => 1,
            ComplexityLevel::Moderate => 2,
            ComplexityLevel::Complex => 3,
            ComplexityLevel::Epic => 4,
        }
    }
}