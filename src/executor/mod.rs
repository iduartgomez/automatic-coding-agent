//! Command execution abstraction layer.
//!
//! Provides a unified interface for executing commands either on the host
//! system or within containers, enabling transparent sandboxed execution.
//!
//! ## Architecture
//!
//! The executor module follows a trait-based design pattern:
//!
//! - [`CommandExecutor`]: Core trait defining command execution interface
//! - [`HostExecutor`]: Executes commands directly on the host system
//! - [`ContainerExecutor`]: Executes commands inside Docker/Podman containers
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use aca::executor::{CommandExecutor, HostExecutor, ExecutionCommand};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let executor = HostExecutor::new();
//!
//!     let command = ExecutionCommand {
//!         program: "echo".to_string(),
//!         args: vec!["Hello, World!".to_string()],
//!         working_dir: None,
//!         env: Default::default(),
//!         stdin: None,
//!         timeout: None,
//!     };
//!
//!     let result = executor.execute(command).await?;
//!     println!("Output: {}", result.stdout);
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub mod config;
pub mod host;
pub mod resources;

#[cfg(feature = "containers")]
pub mod container;

pub use config::{ContainerExecutionConfig, ExecutionMode};
pub use host::HostExecutor;
pub use resources::{ResourceAllocation, SystemResources};

#[cfg(feature = "containers")]
pub use container::ContainerExecutor;

/// Result of command execution
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
