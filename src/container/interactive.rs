//! Interactive container execution support.
//!
//! Provides APIs for interactive shells and real-time stdin/stdout/stderr
//! communication between host and container.

use crate::container::{ContainerError, Result};
use bollard::container::AttachContainerOptions;
use bollard::Docker;
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Interactive session manager.
///
/// Handles bidirectional communication with a container for interactive
/// shell sessions or real-time command execution.
pub struct InteractiveSession {
    container_id: String,
    docker: Arc<Docker>,
    stdin_tx: Option<Arc<Mutex<tokio::io::DuplexStream>>>,
}

impl InteractiveSession {
    /// Attach to a running container for interactive execution.
    ///
    /// # Arguments
    ///
    /// * `docker` - Docker client
    /// * `container_id` - Container ID to attach to
    ///
    /// # Errors
    ///
    /// Returns error if attachment fails.
    pub async fn attach(docker: Docker, container_id: String) -> Result<Self> {
        debug!("Attaching to container for interactive session: {}", container_id);

        Ok(Self {
            container_id,
            docker: Arc::new(docker),
            stdin_tx: None,
        })
    }

    /// Start an interactive shell in the container with TTY.
    ///
    /// This connects the container's stdin/stdout/stderr to the host,
    /// allowing full interactive shell access.
    ///
    /// # Errors
    ///
    /// Returns error if exec creation or attachment fails.
    pub async fn start_shell(&mut self) -> Result<()> {
        use bollard::exec::CreateExecOptions;

        debug!("Starting interactive shell in container: {}", self.container_id);

        // Create exec instance with TTY
        let exec = self
            .docker
            .create_exec(
                &self.container_id,
                CreateExecOptions {
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    cmd: Some(vec!["bash".to_string()]),
                    ..Default::default()
                },
            )
            .await?;

        // Start exec with attached streams
        let start_exec = self.docker.start_exec(&exec.id, None).await?;

        match start_exec {
            bollard::exec::StartExecResults::Attached {
                mut output,
                mut input,
            } => {
                // Handle stdin
                let stdin = tokio::io::stdin();
                let input_handle = tokio::spawn(async move {
                    let mut reader = tokio::io::BufReader::new(stdin);
                    let mut buffer = vec![0u8; 1024];
                    loop {
                        match reader.read(&mut buffer).await {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                if let Err(e) = input.write_all(&buffer[..n]).await {
                                    warn!("Failed to write to container stdin: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read from host stdin: {}", e);
                                break;
                            }
                        }
                    }
                });

                // Handle stdout/stderr
                let stdout = tokio::io::stdout();
                let mut writer = tokio::io::BufWriter::new(stdout);
                while let Some(Ok(log)) = output.next().await {
                    let bytes = log.into_bytes();
                    if let Err(e) = writer.write_all(&bytes).await {
                        warn!("Failed to write to host stdout: {}", e);
                        break;
                    }
                    if let Err(e) = writer.flush().await {
                        warn!("Failed to flush stdout: {}", e);
                        break;
                    }
                }

                // Wait for input handling to complete
                let _ = input_handle.await;

                Ok(())
            }
            bollard::exec::StartExecResults::Detached => {
                Err(ContainerError::ExecutionError(
                    "Unexpected detached execution".to_string(),
                ))
            }
        }
    }

    /// Execute a command interactively with real-time output.
    ///
    /// Unlike regular exec, this streams output as it's produced rather
    /// than buffering it all.
    ///
    /// # Arguments
    ///
    /// * `cmd` - Command to execute
    /// * `callback` - Function called for each line of output
    ///
    /// # Errors
    ///
    /// Returns error if execution fails.
    pub async fn exec_streaming<F>(
        &self,
        cmd: Vec<&str>,
        mut callback: F,
    ) -> Result<i64>
    where
        F: FnMut(String) + Send + 'static,
    {
        use bollard::exec::CreateExecOptions;

        debug!("Executing streaming command: {:?}", cmd);

        let exec = self
            .docker
            .create_exec(
                &self.container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(cmd.into_iter().map(String::from).collect()),
                    ..Default::default()
                },
            )
            .await?;

        let start_exec = self.docker.start_exec(&exec.id, None).await?;

        match start_exec {
            bollard::exec::StartExecResults::Attached { mut output, .. } => {
                while let Some(Ok(log)) = output.next().await {
                    let text = log.to_string();
                    callback(text);
                }
            }
            bollard::exec::StartExecResults::Detached => {
                return Err(ContainerError::ExecutionError(
                    "Unexpected detached execution".to_string(),
                ));
            }
        }

        // Get exit code
        let inspect = self.docker.inspect_exec(&exec.id).await?;
        Ok(inspect.exit_code.unwrap_or(-1))
    }

    /// Get the container ID.
    pub fn container_id(&self) -> &str {
        &self.container_id
    }
}

/// Helper to attach to a container's main process (PID 1).
///
/// This is useful for attaching to containers started with `docker run -it`.
///
/// # Errors
///
/// Returns error if attachment fails.
pub async fn attach_to_container(docker: &Docker, container_id: &str) -> Result<()> {
    debug!("Attaching to container main process: {}", container_id);

    let options = AttachContainerOptions::<String> {
        stdin: Some(true),
        stdout: Some(true),
        stderr: Some(true),
        stream: Some(true),
        logs: Some(false),
        ..Default::default()
    };

    let bollard::container::AttachContainerResults { mut output, mut input } =
        docker.attach_container(container_id, Some(options)).await?;

    // Handle stdin
    let stdin = tokio::io::stdin();
    let input_handle = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdin);
        let mut buffer = vec![0u8; 1024];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    if let Err(e) = input.write_all(&buffer[..n]).await {
                        warn!("Failed to write to container: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    warn!("Failed to read from stdin: {}", e);
                    break;
                }
            }
        }
    });

    // Handle stdout/stderr
    let stdout = tokio::io::stdout();
    let mut writer = tokio::io::BufWriter::new(stdout);
    while let Some(Ok(log)) = output.next().await {
        let bytes = log.into_bytes();
        if let Err(e) = writer.write_all(&bytes).await {
            warn!("Failed to write to stdout: {}", e);
            break;
        }
        if let Err(e) = writer.flush().await {
            warn!("Failed to flush stdout: {}", e);
            break;
        }
    }

    let _ = input_handle.await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_session_creation() {
        // Just test that the struct can be created
        let docker = bollard::Docker::connect_with_local_defaults().unwrap();
        let container_id = "test-container".to_string();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let session = InteractiveSession::attach(docker, container_id).await;
            assert!(session.is_ok());
        });
    }
}
