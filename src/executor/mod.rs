//! # Command Execution Abstraction Layer
//!
//! Provides unified command execution across host and containerized environments,
//! enabling transparent sandboxed execution with resource management and timeout handling.
//!
//! ## Core Components
//!
//! - **[`CommandExecutor`]**: Unified enum executor supporting both host and container modes
//! - **[`HostExecutor`]**: Direct host system command execution with process management
//! - **[`ContainerExecutor`]**: Sandboxed execution within Docker/Podman containers
//! - **[`ExecutionCommand`]**: Command specification with arguments, environment, and timeouts
//! - **[`ExecutionResult`]**: Execution outcome with stdout, stderr, exit code, and duration
//! - **[`SystemResources`]**: System resource detection and allocation for containers
//!
//! ## Key Features
//!
//! ### 🖥️ Host Execution
//! - Direct process spawning via `tokio::process::Command`
//! - Full environment variable control
//! - Working directory management
//! - Configurable stdin piping
//! - Timeout support with automatic process termination
//!
//! ### 🐳 Container Execution
//! - Isolated execution within Docker/Podman containers
//! - Automatic resource allocation (CPU, memory)
//! - Volume mounting for workspace and session data
//! - Network isolation and security
//! - Container lifecycle management (auto-remove support)
//!
//! ### ⏱️ Timeout Handling
//! - Per-command timeout configuration
//! - Graceful process termination on timeout
//! - Automatic cleanup of zombie processes
//! - Clear timeout error reporting
//!
//! ### 📊 Resource Management
//! - Automatic system resource detection
//! - Percentage-based container resource allocation
//! - Explicit memory and CPU quota override
//! - Cross-platform resource detection (Linux, macOS, Windows)
//!
//! ### 🔒 Error Handling
//! - Comprehensive error types for different failure modes
//! - Container unavailability detection
//! - Command execution failures with detailed messages
//! - I/O error propagation
//!
//! ## Execution Flow
//!
//! ```text
//! ExecutionCommand
//!        ↓
//!   CommandExecutor::execute()
//!        ↓
//!   ┌────┴────┐
//!   │         │
//! Host    Container
//!   │         │
//!   ↓         ↓
//! Process   Docker/Podman
//!   │         │
//!   └────┬────┘
//!        ↓
//!  ExecutionResult
//! ```
//!
//! ## Example Usage
//!
//! ### Basic Host Execution
//!
//! ```rust,no_run
//! use aca::executor::{CommandExecutor, HostExecutor, ExecutionCommand};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let executor = CommandExecutor::Host(HostExecutor::new());
//!
//!     let command = ExecutionCommand::new("echo", vec!["Hello, World!".to_string()]);
//!
//!     let result = executor.execute(command).await?;
//!     println!("Output: {}", result.stdout);
//!     println!("Exit code: {}", result.exit_code);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Container Execution with Resource Limits
//!
//! ```rust,no_run
//! use aca::executor::{CommandExecutor, ExecutionCommand};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     # #[cfg(feature = "containers")]
//!     # {
//!     use aca::executor::{ContainerExecutor, container::ContainerExecutorConfig};
//!
//!     let config = ContainerExecutorConfig {
//!         image: "alpine:latest".to_string(),
//!         workspace_mount: PathBuf::from("/workspace"),
//!         aca_mount: PathBuf::from("/workspace/.aca"),
//!         memory_bytes: Some(512_000_000), // 512 MB
//!         cpu_quota: Some(50_000),         // 50% CPU
//!         auto_remove: true,
//!     };
//!
//!     let executor = CommandExecutor::Container(
//!         ContainerExecutor::new(config).await?
//!     );
//!
//!     let command = ExecutionCommand::new("ls", vec!["-la".to_string()])
//!         .with_working_dir(PathBuf::from("/workspace"));
//!
//!     let result = executor.execute(command).await?;
//!     println!("Files: {}", result.stdout);
//!     # }
//!     Ok(())
//! }
//! ```
//!
//! ### Command with Timeout
//!
//! ```rust,no_run
//! use aca::executor::{CommandExecutor, HostExecutor, ExecutionCommand};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let executor = CommandExecutor::Host(HostExecutor::new());
//!
//!     let command = ExecutionCommand::new("sleep", vec!["10".to_string()])
//!         .with_timeout(Duration::from_secs(2)); // Timeout after 2 seconds
//!
//!     match executor.execute(command).await {
//!         Ok(result) => println!("Completed: {}", result.stdout),
//!         Err(e) => eprintln!("Timed out: {}", e),
//!     }
//!
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Execution mode configuration and runtime settings.
///
/// Defines the [`ExecutionMode`] enum and [`ContainerExecutionConfig`]
/// for configuring how commands are executed (host vs container).
pub mod config;

/// Host-based command execution.
///
/// Implements [`HostExecutor`] for direct process execution on the
/// host system using `tokio::process::Command`.
pub mod host;

/// System resource detection and allocation.
///
/// Provides [`SystemResources`] for detecting available CPU and memory,
/// and [`ResourceAllocation`] for calculating container resource limits.
pub mod resources;

/// Container-based command execution (requires `containers` feature).
///
/// Implements [`ContainerExecutor`] for sandboxed execution within
/// Docker or Podman containers with resource limits and volume mounts.
#[cfg(feature = "containers")]
pub mod container;

pub use config::{ContainerExecutionConfig, ExecutionMode};
pub use host::HostExecutor;
pub use resources::{ResourceAllocation, SystemResources};

#[cfg(feature = "containers")]
pub use container::ContainerExecutor;

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Standard output from the command
    pub stdout: String,
    /// Standard error from the command
    pub stderr: String,
    /// Exit code (0 = success, non-zero = failure)
    pub exit_code: i32,
    /// Duration of command execution
    pub duration: Duration,
}

impl ExecutionResult {
    /// Check if the command executed successfully (exit code 0)
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Command to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCommand {
    /// Program name or path to execute
    pub program: String,
    /// Command line arguments
    pub args: Vec<String>,
    /// Working directory for command execution
    pub working_dir: Option<PathBuf>,
    /// Environment variables to set
    pub env: HashMap<String, String>,
    /// Standard input to provide to the command
    pub stdin: Option<String>,
    /// Maximum execution time (None = no timeout)
    pub timeout: Option<Duration>,
}

impl ExecutionCommand {
    /// Create a new command with just program and args
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
            working_dir: None,
            env: HashMap::new(),
            stdin: None,
            timeout: None,
        }
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set standard input
    pub fn with_stdin(mut self, stdin: impl Into<String>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    /// Set execution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Errors during command execution
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    /// Container runtime is unavailable
    #[error("Container unavailable: {0}")]
    ContainerUnavailable(String),

    /// Command execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Command execution timed out
    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    /// Container-specific error
    #[cfg(feature = "containers")]
    #[error("Container error: {0}")]
    ContainerError(#[from] crate::container::ContainerError),

    /// I/O error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Other error
    #[error("{0}")]
    Other(String),
}

/// Command executor enum - abstracts where commands run
///
/// This enum provides a unified interface for executing commands either on the host
/// or within containers. Using an enum instead of a trait allows for better
/// performance (no vtable overhead) and is more idiomatic Rust.
#[derive(Clone)]
pub enum CommandExecutor {
    /// Execute commands on the host system
    Host(host::HostExecutor),
    /// Execute commands in a container
    #[cfg(feature = "containers")]
    Container(container::ContainerExecutor),
}

impl CommandExecutor {
    /// Execute a command and return the result
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to execute, times out,
    /// or if the executor is unavailable.
    pub async fn execute(&self, command: ExecutionCommand) -> Result<ExecutionResult, ExecutorError> {
        match self {
            Self::Host(executor) => executor.execute(command).await,
            #[cfg(feature = "containers")]
            Self::Container(executor) => executor.execute(command).await,
        }
    }

    /// Check if the executor is available and healthy
    ///
    /// # Errors
    ///
    /// Returns an error if the executor is not available or unhealthy.
    pub async fn health_check(&self) -> Result<(), ExecutorError> {
        match self {
            Self::Host(executor) => executor.health_check().await,
            #[cfg(feature = "containers")]
            Self::Container(executor) => executor.health_check().await,
        }
    }

    /// Get executor type name for logging
    pub fn executor_type(&self) -> &'static str {
        match self {
            Self::Host(executor) => executor.executor_type(),
            #[cfg(feature = "containers")]
            Self::Container(executor) => executor.executor_type(),
        }
    }

    /// Cleanup resources (called on shutdown)
    ///
    /// # Errors
    ///
    /// Returns an error if cleanup fails.
    pub async fn shutdown(&self) -> Result<(), ExecutorError> {
        match self {
            Self::Host(executor) => executor.shutdown().await,
            #[cfg(feature = "containers")]
            Self::Container(executor) => executor.shutdown().await,
        }
    }
}
