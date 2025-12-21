//! Container-based command execution.
//!
//! Executes commands inside Docker/Podman containers for isolated execution.

use super::{ExecutionCommand, ExecutionResult, ExecutorError};
use crate::container::{ContainerConfig, ContainerOrchestrator, ExecConfig};
use crate::executor::config::DEFAULT_CONTAINER_IMAGE;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Container execution configuration
#[derive(Debug, Clone)]
pub struct ContainerExecutorConfig {
    /// Container image to use
    pub image: String,
    /// Workspace mount path (host)
    pub workspace_mount: PathBuf,
    /// ACA directory mount path (host)
    pub aca_mount: PathBuf,
    /// Memory limit in bytes
    pub memory_bytes: Option<i64>,
    /// CPU quota (microseconds per period)
    pub cpu_quota: Option<i64>,
    /// Auto-remove container on shutdown
    pub auto_remove: bool,
}

impl Default for ContainerExecutorConfig {
    fn default() -> Self {
        Self {
            image: DEFAULT_CONTAINER_IMAGE.to_string(),
            workspace_mount: PathBuf::new(),
            aca_mount: PathBuf::new(),
            memory_bytes: None,
            cpu_quota: None,
            auto_remove: true,
        }
    }
}

/// Executes commands inside a container
#[derive(Clone)]
pub struct ContainerExecutor {
    orchestrator: Arc<ContainerOrchestrator>,
    config: ContainerExecutorConfig,
    container_id: Arc<RwLock<Option<String>>>,
}

impl ContainerExecutor {
    /// Create a new container executor
    ///
    /// # Errors
    ///
    /// Returns an error if the container runtime is unavailable.
    pub async fn new(config: ContainerExecutorConfig) -> Result<Self, ExecutorError> {
        let orchestrator = ContainerOrchestrator::new()
            .await
            .map_err(|e| ExecutorError::ContainerUnavailable(e.to_string()))?;

        Ok(Self {
            orchestrator: Arc::new(orchestrator),
            config,
            container_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Ensure container is running, creating if necessary
    async fn ensure_container(&self) -> Result<String, ExecutorError> {
        let mut id_guard = self.container_id.write().await;

        if let Some(ref id) = *id_guard {
            return Ok(id.clone());
        }

        info!(
            "Creating session container with image: {}",
            self.config.image
        );

        // Build container configuration
        let mut container_config = ContainerConfig::builder()
            .image(&self.config.image)
            .cmd(vec!["sleep", "infinity"])
            .working_dir("/workspace");

        // Mount workspace
        if !self.config.workspace_mount.as_os_str().is_empty() {
            container_config = container_config.bind(format!(
                "{}:/workspace:rw",
                self.config.workspace_mount.display()
            ));
        }

        // Mount .aca directory
        if !self.config.aca_mount.as_os_str().is_empty() {
            container_config =
                container_config.bind(format!("{}:/.aca:rw", self.config.aca_mount.display()));
        }

        // Apply resource limits if configured
        if let Some(mem) = self.config.memory_bytes {
            container_config = container_config.memory_limit(mem);
        }

        if let Some(cpu) = self.config.cpu_quota {
            container_config = container_config.cpu_quota(cpu);
        }

        let container_config = container_config
            .build()
            .map_err(|e| ExecutorError::Other(format!("Container config error: {}", e)))?;

        // Create and start container
        let container_id = self
            .orchestrator
            .create_container(&container_config, Some("aca-session"))
            .await?;

        self.orchestrator.start_container(&container_id).await?;

        info!(
            "Container started: {}",
            container_id.get(..12).unwrap_or(&container_id)
        );

        *id_guard = Some(container_id.clone());
        Ok(container_id)
    }

    /// Get the container ID if it exists
    pub async fn container_id(&self) -> Option<String> {
        self.container_id.read().await.clone()
    }
}

impl ContainerExecutor {
    pub async fn execute(&self, cmd: ExecutionCommand) -> Result<ExecutionResult, ExecutorError> {
        let container_id = self.ensure_container().await?;

        debug!(
            "Executing command in container {}: {} {:?}",
            container_id.get(..12).unwrap_or(&container_id),
            cmd.program,
            cmd.args
        );

        let start = Instant::now();

        // Build exec configuration
        let mut exec_args = vec![cmd.program.clone()];
        exec_args.extend(cmd.args.clone());

        let mut exec_config = ExecConfig::builder()
            .cmd(exec_args.iter().map(|s| s.as_str()))
            .attach_stdout(true)
            .attach_stderr(true);

        // Set working directory if specified
        if let Some(ref dir) = cmd.working_dir {
            exec_config = exec_config.working_dir(dir.to_string_lossy().to_string());
        }

        // Add environment variables
        for (key, value) in &cmd.env {
            exec_config = exec_config.env(key, value);
        }

        let exec_config = exec_config.build();

        // Execute command with optional timeout
        let output = if let Some(timeout) = cmd.timeout {
            match tokio::time::timeout(
                timeout,
                self.orchestrator
                    .exec_with_config(&container_id, &exec_config),
            )
            .await
            {
                Ok(result) => result?,
                Err(_) => {
                    return Err(ExecutorError::Timeout(timeout));
                }
            }
        } else {
            self.orchestrator
                .exec_with_config(&container_id, &exec_config)
                .await?
        };

        let duration = start.elapsed();

        Ok(ExecutionResult {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.exit_code.unwrap_or(-1) as i32,
            duration,
        })
    }

    pub async fn health_check(&self) -> Result<(), ExecutorError> {
        // Check if we can connect to the container runtime
        self.orchestrator
            .client()
            .ping()
            .await
            .map_err(|e| ExecutorError::ContainerUnavailable(e.to_string()))
    }

    pub fn executor_type(&self) -> &'static str {
        "container"
    }

    pub async fn shutdown(&self) -> Result<(), ExecutorError> {
        let mut id_guard = self.container_id.write().await;
        if let Some(ref id) = *id_guard {
            info!(
                "Stopping and removing container: {}",
                id.get(..12).unwrap_or(id)
            );
            self.orchestrator.stop_and_remove(id).await?;
            *id_guard = None; // Clear the container ID after successful removal
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Helper to check if containers should be tested
    fn should_run_container_tests() -> bool {
        std::env::var("SKIP_CONTAINER_TESTS")
            .map(|v| v != "1")
            .unwrap_or(true)
    }

    #[tokio::test]
    #[ignore] // Requires Docker/Podman
    async fn test_container_executor_creation() {
        if !should_run_container_tests() {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let config = ContainerExecutorConfig {
            image: "alpine:latest".to_string(),
            workspace_mount: temp_dir.path().to_path_buf(),
            aca_mount: temp_dir.path().join(".aca"),
            ..Default::default()
        };

        let executor = ContainerExecutor::new(config).await;
        assert!(executor.is_ok());

        let executor = executor.unwrap();
        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Docker/Podman
    async fn test_container_executor_simple_command() {
        if !should_run_container_tests() {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let config = ContainerExecutorConfig {
            image: "alpine:latest".to_string(),
            workspace_mount: temp_dir.path().to_path_buf(),
            aca_mount: temp_dir.path().join(".aca"),
            ..Default::default()
        };

        let executor = ContainerExecutor::new(config).await.unwrap();

        let cmd = ExecutionCommand::new("echo", vec!["hello".to_string()]);
        let result = executor.execute(cmd).await.unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Docker/Podman
    async fn test_container_executor_health_check() {
        if !should_run_container_tests() {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let config = ContainerExecutorConfig {
            image: "alpine:latest".to_string(),
            workspace_mount: temp_dir.path().to_path_buf(),
            aca_mount: temp_dir.path().join(".aca"),
            ..Default::default()
        };

        let executor = ContainerExecutor::new(config).await.unwrap();
        assert!(executor.health_check().await.is_ok());

        executor.shutdown().await.unwrap();
    }
}
