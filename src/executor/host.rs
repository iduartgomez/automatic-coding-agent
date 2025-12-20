//! Native host command execution.
//!
//! Executes commands directly on the host system using `tokio::process::Command`.

use super::{CommandExecutor, ExecutionCommand, ExecutionResult, ExecutorError};
use async_trait::async_trait;
use std::time::Instant;
use tokio::process::Command;
use tracing::debug;

/// Executes commands directly on the host system
#[derive(Debug, Clone)]
pub struct HostExecutor;

impl HostExecutor {
    /// Create a new host executor
    pub fn new() -> Self {
        Self
    }
}

impl Default for HostExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandExecutor for HostExecutor {
    async fn execute(&self, cmd: ExecutionCommand) -> Result<ExecutionResult, ExecutorError> {
        debug!(
            "Executing command on host: {} {:?}",
            cmd.program, cmd.args
        );

        let start = Instant::now();

        let mut command = Command::new(&cmd.program);
        command.args(&cmd.args);

        if let Some(ref dir) = cmd.working_dir {
            command.current_dir(dir);
        }

        for (key, value) in &cmd.env {
            command.env(key, value);
        }

        // Execute command with optional timeout
        let output = if let Some(timeout) = cmd.timeout {
            match tokio::time::timeout(timeout, command.output()).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(ExecutorError::Timeout(timeout));
                }
            }
        } else {
            command.output().await?
        };

        let duration = start.elapsed();

        Ok(ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration,
        })
    }

    async fn health_check(&self) -> Result<(), ExecutorError> {
        // Host is always available
        Ok(())
    }

    fn executor_type(&self) -> &'static str {
        "host"
    }

    async fn shutdown(&self) -> Result<(), ExecutorError> {
        // No resources to clean up
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_host_executor_simple_command() {
        let executor = HostExecutor::new();

        let cmd = ExecutionCommand::new("echo", vec!["hello".to_string()]);

        let result = executor.execute(cmd).await.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
        assert!(result.success());
    }

    #[tokio::test]
    async fn test_host_executor_with_args() {
        let executor = HostExecutor::new();

        let cmd = ExecutionCommand::new("echo", vec!["hello".to_string(), "world".to_string()]);

        let result = executor.execute(cmd).await.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn test_host_executor_working_directory() {
        let executor = HostExecutor::new();

        let cmd = ExecutionCommand::new("pwd", vec![]).with_working_dir(PathBuf::from("/tmp"));

        let result = executor.execute(cmd).await.unwrap();
        assert_eq!(result.exit_code, 0);
        #[cfg(not(target_os = "windows"))]
        assert!(result.stdout.contains("/tmp") || result.stdout.contains("/private/tmp"));
    }

    #[tokio::test]
    async fn test_host_executor_environment_variable() {
        let executor = HostExecutor::new();

        #[cfg(not(target_os = "windows"))]
        let cmd = ExecutionCommand::new("sh", vec!["-c".to_string(), "echo $TEST_VAR".to_string()])
            .with_env("TEST_VAR", "hello");

        #[cfg(target_os = "windows")]
        let cmd =
            ExecutionCommand::new("cmd", vec!["/c".to_string(), "echo %TEST_VAR%".to_string()])
                .with_env("TEST_VAR", "hello");

        let result = executor.execute(cmd).await.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_host_executor_timeout() {
        let executor = HostExecutor::new();

        #[cfg(not(target_os = "windows"))]
        let cmd =
            ExecutionCommand::new("sleep", vec!["2".to_string()]).with_timeout(std::time::Duration::from_millis(100));

        #[cfg(target_os = "windows")]
        let cmd = ExecutionCommand::new("timeout", vec!["/t".to_string(), "2".to_string()])
            .with_timeout(std::time::Duration::from_millis(100));

        let result = executor.execute(cmd).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExecutorError::Timeout(_)));
    }

    #[tokio::test]
    async fn test_host_executor_health_check() {
        let executor = HostExecutor::new();
        assert!(executor.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_host_executor_type() {
        let executor = HostExecutor::new();
        assert_eq!(executor.executor_type(), "host");
    }
}
