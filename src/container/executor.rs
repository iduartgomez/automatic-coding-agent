//! Container command execution.
//!
//! Provides APIs for executing commands in running containers with
//! stdin/stdout/stderr handling.

use crate::container::{ContainerError, Result};
use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecResults};
use futures::stream::StreamExt;
use std::default::Default;
use tracing::debug;

/// Execution configuration builder.
pub struct ExecConfigBuilder {
    cmd: Vec<String>,
    env: Vec<String>,
    working_dir: Option<String>,
    user: Option<String>,
    attach_stdin: bool,
    attach_stdout: bool,
    attach_stderr: bool,
    tty: bool,
    privileged: bool,
}

impl Default for ExecConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecConfigBuilder {
    /// Create a new execution configuration builder.
    pub fn new() -> Self {
        Self {
            cmd: Vec::new(),
            env: Vec::new(),
            working_dir: None,
            user: None,
            attach_stdin: false,
            attach_stdout: true, // Default to true to capture output
            attach_stderr: true, // Default to true to capture errors
            tty: false,
            privileged: false,
        }
    }

    /// Set the command to execute.
    pub fn cmd<I, S>(mut self, cmd: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.cmd = cmd.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Add an environment variable.
    pub fn env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env.push(format!("{}={}", key.into(), value.into()));
        self
    }

    /// Set the working directory.
    pub fn working_dir<S: Into<String>>(mut self, dir: S) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set the user to execute as.
    pub fn user<S: Into<String>>(mut self, user: S) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Attach to stdin.
    pub fn attach_stdin(mut self, attach: bool) -> Self {
        self.attach_stdin = attach;
        self
    }

    /// Attach to stdout.
    pub fn attach_stdout(mut self, attach: bool) -> Self {
        self.attach_stdout = attach;
        self
    }

    /// Attach to stderr.
    pub fn attach_stderr(mut self, attach: bool) -> Self {
        self.attach_stderr = attach;
        self
    }

    /// Enable TTY allocation.
    pub fn tty(mut self, enable: bool) -> Self {
        self.tty = enable;
        self
    }

    /// Run with elevated privileges.
    pub fn privileged(mut self, enable: bool) -> Self {
        self.privileged = enable;
        self
    }

    /// Build the execution configuration.
    pub fn build(self) -> ExecConfig {
        ExecConfig {
            cmd: self.cmd,
            env: self.env,
            working_dir: self.working_dir,
            user: self.user,
            attach_stdin: self.attach_stdin,
            attach_stdout: self.attach_stdout,
            attach_stderr: self.attach_stderr,
            tty: self.tty,
            privileged: self.privileged,
        }
    }
}

/// Container execution configuration.
#[derive(Debug, Clone)]
pub struct ExecConfig {
    cmd: Vec<String>,
    env: Vec<String>,
    working_dir: Option<String>,
    user: Option<String>,
    attach_stdin: bool,
    attach_stdout: bool,
    attach_stderr: bool,
    tty: bool,
    privileged: bool,
}

impl ExecConfig {
    /// Create a new execution configuration builder.
    pub fn builder() -> ExecConfigBuilder {
        ExecConfigBuilder::new()
    }

    /// Get the command.
    pub fn cmd(&self) -> &[String] {
        &self.cmd
    }
}

/// Output from command execution.
#[derive(Debug, Clone)]
pub struct ExecOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code (None if not available)
    pub exit_code: Option<i64>,
}

impl ExecOutput {
    /// Check if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Get combined output (stdout + stderr).
    pub fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Execute a command in a running container.
///
/// # Errors
///
/// Returns error if execution fails or container not found.
pub async fn execute(
    docker: &Docker,
    container_id: &str,
    config: &ExecConfig,
) -> Result<ExecOutput> {
    debug!(
        "Executing command in container {}: {:?}",
        container_id, config.cmd
    );

    // Create exec instance
    let exec_options = CreateExecOptions {
        cmd: Some(config.cmd.clone()),
        env: if config.env.is_empty() {
            None
        } else {
            Some(config.env.clone())
        },
        working_dir: config.working_dir.clone(),
        user: config.user.clone(),
        attach_stdin: Some(config.attach_stdin),
        attach_stdout: Some(config.attach_stdout),
        attach_stderr: Some(config.attach_stderr),
        tty: Some(config.tty),
        privileged: Some(config.privileged),
        ..Default::default()
    };

    let exec = docker.create_exec(container_id, exec_options).await?;

    // Start exec
    let start_results = docker.start_exec(&exec.id, None).await?;

    let mut stdout = String::new();
    let mut stderr = String::new();

    // Collect output
    match start_results {
        StartExecResults::Attached { mut output, .. } => {
            while let Some(result) = output.next().await {
                match result {
                    Ok(log) => {
                        let text = log.to_string();
                        match log {
                            bollard::container::LogOutput::StdOut { .. } => {
                                stdout.push_str(&text);
                            }
                            bollard::container::LogOutput::StdErr { .. } => {
                                stderr.push_str(&text);
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        return Err(ContainerError::ExecutionError(format!(
                            "Failed to read output: {}",
                            e
                        )));
                    }
                }
            }
        }
        StartExecResults::Detached => {
            return Err(ContainerError::ExecutionError(
                "Unexpected detached execution".to_string(),
            ));
        }
    }

    // Get exit code
    let inspect = docker.inspect_exec(&exec.id).await?;
    let exit_code = inspect.exit_code;

    debug!("Command executed with exit code: {:?}", exit_code);

    Ok(ExecOutput {
        stdout,
        stderr,
        exit_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_config_builder() {
        let config = ExecConfig::builder()
            .cmd(vec!["echo", "hello"])
            .env("FOO", "bar")
            .working_dir("/tmp")
            .user("root")
            .attach_stdout(true)
            .build();

        assert_eq!(config.cmd(), &["echo", "hello"]);
        assert_eq!(config.env, vec!["FOO=bar"]);
        assert_eq!(config.working_dir, Some("/tmp".to_string()));
        assert_eq!(config.user, Some("root".to_string()));
        assert!(config.attach_stdout);
    }

    #[test]
    fn test_exec_output_success() {
        let output = ExecOutput {
            stdout: "hello\n".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
        };

        assert!(output.success());
        assert_eq!(output.combined(), "hello\n");
    }

    #[test]
    fn test_exec_output_failure() {
        let output = ExecOutput {
            stdout: String::new(),
            stderr: "error\n".to_string(),
            exit_code: Some(1),
        };

        assert!(!output.success());
        assert_eq!(output.combined(), "error\n");
    }
}
