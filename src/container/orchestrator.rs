//! Container lifecycle orchestration.
//!
//! Provides high-level container management including creation, startup,
//! execution, monitoring, and cleanup.

use crate::container::{
    ContainerClient, ContainerConfig, ContainerError, ExecConfig, ExecOutput, Result,
};
use futures::stream::StreamExt;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Container orchestrator configuration.
#[derive(Debug, Clone)]
pub struct ContainerOrchestratorConfig {
    /// Automatically pull images if not present
    pub auto_pull: bool,
    /// Container name prefix
    pub name_prefix: String,
    /// Default stop timeout in seconds
    pub stop_timeout: i64,
}

impl Default for ContainerOrchestratorConfig {
    fn default() -> Self {
        Self {
            auto_pull: true,
            name_prefix: "aca".to_string(),
            stop_timeout: 10,
        }
    }
}

/// High-level container orchestrator.
///
/// Manages container lifecycle including image pulling, container creation,
/// execution, and cleanup operations.
pub struct ContainerOrchestrator {
    client: ContainerClient,
    config: ContainerOrchestratorConfig,
}

impl ContainerOrchestrator {
    /// Create a new orchestrator with default configuration.
    ///
    /// # Errors
    ///
    /// Returns error if connection to container runtime fails.
    pub async fn new() -> Result<Self> {
        Self::with_config(ContainerOrchestratorConfig::default()).await
    }

    /// Create a new orchestrator with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns error if connection to container runtime fails.
    pub async fn with_config(config: ContainerOrchestratorConfig) -> Result<Self> {
        let client = ContainerClient::new().await?;
        Ok(Self { client, config })
    }

    /// Create an orchestrator with an existing client.
    pub fn with_client(client: ContainerClient, config: ContainerOrchestratorConfig) -> Self {
        Self { client, config }
    }

    /// Pull a container image if not present locally.
    ///
    /// # Errors
    ///
    /// Returns error if image pull fails.
    pub async fn ensure_image(&self, image: &str) -> Result<()> {
        // Check if image exists locally
        if self.client.image_exists(image).await? {
            debug!("Image {} already exists locally", image);
            return Ok(());
        }

        info!("Pulling image: {}", image);
        self.pull_image(image).await
    }

    /// Pull a container image from registry.
    ///
    /// # Errors
    ///
    /// Returns error if image pull fails.
    pub async fn pull_image(&self, image: &str) -> Result<()> {
        let mut stream = self.client.docker().create_image(
            Some(bollard::image::CreateImageOptions {
                from_image: image,
                ..Default::default()
            }),
            None,
            None,
        );

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!("Pull status: {}", status);
                    }
                    if let Some(progress) = info.progress {
                        debug!("Pull progress: {}", progress);
                    }
                }
                Err(e) => {
                    return Err(ContainerError::ApiError(e));
                }
            }
        }

        info!("Successfully pulled image: {}", image);
        Ok(())
    }

    /// Create a container from configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Container configuration
    /// * `name` - Optional container name (auto-generated if None)
    ///
    /// # Returns
    ///
    /// Container ID
    ///
    /// # Errors
    ///
    /// Returns error if container creation fails.
    pub async fn create_container(
        &self,
        config: &ContainerConfig,
        name: Option<&str>,
    ) -> Result<String> {
        // Ensure image is available
        if self.config.auto_pull {
            if let Some(image) = config.image() {
                self.ensure_image(image).await?;
            }
        }

        // Generate container name if not provided
        let container_name = name
            .map(String::from)
            .unwrap_or_else(|| format!("{}-{}", self.config.name_prefix, uuid::Uuid::new_v4()));

        let options = bollard::container::CreateContainerOptions {
            name: container_name.as_str(),
            ..Default::default()
        };

        debug!("Creating container: {}", container_name);

        use bollard::container::Config as BollardConfig;

        let bollard_config = BollardConfig {
            image: Some(config.image.clone()),
            cmd: config.cmd.clone(),
            entrypoint: config.entrypoint.clone(),
            working_dir: config.working_dir.clone(),
            env: config.env.clone(),
            labels: config.labels.clone(),
            user: config.user.clone(),
            host_config: Some(config.host_config.clone()),
            ..Default::default()
        };

        let response = self
            .client
            .docker()
            .create_container(Some(options), bollard_config)
            .await?;

        info!("Created container: {} ({})", container_name, response.id);

        Ok(response.id)
    }

    /// Start a container.
    ///
    /// # Errors
    ///
    /// Returns error if container start fails.
    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        debug!("Starting container: {}", container_id);

        self.client
            .docker()
            .start_container(
                container_id,
                None::<bollard::container::StartContainerOptions<String>>,
            )
            .await?;

        info!("Started container: {}", container_id);
        Ok(())
    }

    /// Stop a container.
    ///
    /// # Errors
    ///
    /// Returns error if container stop fails.
    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        debug!("Stopping container: {}", container_id);

        self.client
            .docker()
            .stop_container(
                container_id,
                Some(bollard::container::StopContainerOptions {
                    t: self.config.stop_timeout,
                }),
            )
            .await?;

        info!("Stopped container: {}", container_id);
        Ok(())
    }

    /// Remove a container.
    ///
    /// # Errors
    ///
    /// Returns error if container removal fails.
    pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        debug!("Removing container: {}", container_id);

        self.client
            .docker()
            .remove_container(
                container_id,
                Some(bollard::container::RemoveContainerOptions {
                    force,
                    v: true, // Remove associated volumes
                    ..Default::default()
                }),
            )
            .await?;

        info!("Removed container: {}", container_id);
        Ok(())
    }

    /// Stop and remove a container.
    ///
    /// # Errors
    ///
    /// Returns error if stop or removal fails.
    pub async fn stop_and_remove(&self, container_id: &str) -> Result<()> {
        // Try to stop first, but don't fail if already stopped
        if let Err(e) = self.stop_container(container_id).await {
            warn!("Failed to stop container {}: {}", container_id, e);
        }

        self.remove_container(container_id, true).await
    }

    /// Execute a command in a running container.
    ///
    /// # Errors
    ///
    /// Returns error if execution fails.
    pub async fn exec(&self, container_id: &str, cmd: Vec<&str>) -> Result<ExecOutput> {
        let config = ExecConfig::builder()
            .cmd(cmd)
            .attach_stdout(true)
            .attach_stderr(true)
            .build();

        self.exec_with_config(container_id, &config).await
    }

    /// Execute a command with full configuration.
    ///
    /// # Errors
    ///
    /// Returns error if execution fails.
    pub async fn exec_with_config(
        &self,
        container_id: &str,
        config: &ExecConfig,
    ) -> Result<ExecOutput> {
        use crate::container::executor;
        executor::execute(self.client.docker(), container_id, config).await
    }

    /// Get container logs.
    ///
    /// # Errors
    ///
    /// Returns error if log retrieval fails.
    pub async fn logs(&self, container_id: &str, tail: Option<&str>) -> Result<String> {
        let mut stream = self.client.docker().logs(
            container_id,
            Some(bollard::container::LogsOptions {
                stdout: true,
                stderr: true,
                tail: tail.unwrap_or("all").to_string(),
                ..Default::default()
            }),
        );
        let mut output = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(log) => {
                    output.push_str(&log.to_string());
                }
                Err(e) => {
                    return Err(ContainerError::ApiError(e));
                }
            }
        }

        Ok(output)
    }

    /// List all containers with optional filters.
    ///
    /// # Errors
    ///
    /// Returns error if listing fails.
    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>> {
        let mut filters = HashMap::new();
        if !all {
            filters.insert("status".to_string(), vec!["running".to_string()]);
        }

        let containers = self
            .client
            .docker()
            .list_containers(Some(bollard::container::ListContainersOptions {
                all,
                filters,
                ..Default::default()
            }))
            .await?;

        Ok(containers
            .into_iter()
            .map(|c| ContainerSummary {
                id: c.id.unwrap_or_else(|| "".to_string()),
                names: c.names.unwrap_or_default(),
                image: c.image.unwrap_or_else(|| "".to_string()),
                state: c
                    .state
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "".to_string()),
                status: c.status.unwrap_or_else(|| "".to_string()),
            })
            .collect())
    }

    /// Get the underlying client.
    pub fn client(&self) -> &ContainerClient {
        &self.client
    }
}

/// Container summary information.
#[derive(Debug, Clone)]
pub struct ContainerSummary {
    /// Container ID
    pub id: String,
    /// Container names
    pub names: Vec<String>,
    /// Image name
    pub image: String,
    /// Container state
    pub state: String,
    /// Container status
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Docker/Podman
    async fn test_orchestrator_creation() {
        let orchestrator = ContainerOrchestrator::new().await.unwrap();
        assert!(orchestrator.client.ping().await.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_image_pull() {
        let orchestrator = ContainerOrchestrator::new().await.unwrap();
        orchestrator.ensure_image("alpine:latest").await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_container_lifecycle() {
        let orchestrator = ContainerOrchestrator::new().await.unwrap();

        // Create container
        let config = ContainerConfig::builder()
            .image("alpine:latest")
            .cmd(vec!["sleep", "infinity"])
            .build()
            .unwrap();

        let container_id = orchestrator.create_container(&config, None).await.unwrap();

        // Start container
        orchestrator.start_container(&container_id).await.unwrap();

        // Execute command
        let output = orchestrator
            .exec(&container_id, vec!["echo", "hello"])
            .await
            .unwrap();
        assert!(output.stdout.contains("hello"));

        // Cleanup
        orchestrator.stop_and_remove(&container_id).await.unwrap();
    }
}
