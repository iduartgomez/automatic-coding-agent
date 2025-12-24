//! Docker/Podman client wrapper.
//!
//! Provides a simplified interface to the bollard Docker API with automatic
//! connection handling, fallback strategies, and health checking.

use crate::container::{ContainerError, Result};
use bollard::Docker;
use std::sync::Arc;
use tracing::{debug, info};

/// Container client configuration.
#[derive(Debug, Clone)]
pub struct ContainerClientConfig {
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Number of connection retries
    pub retries: u32,
}

impl Default for ContainerClientConfig {
    fn default() -> Self {
        Self {
            timeout: 120,
            retries: 3,
        }
    }
}

/// Docker/Podman API client wrapper.
///
/// Manages connection to Docker or Podman daemon with automatic fallback
/// and health checking.
#[derive(Clone)]
pub struct ContainerClient {
    docker: Arc<Docker>,
    #[allow(dead_code)] // Reserved for future configuration options
    config: ContainerClientConfig,
}

impl ContainerClient {
    /// Create a new container client with default configuration.
    ///
    /// Attempts to connect to Docker first, then falls back to Podman if available.
    ///
    /// # Errors
    ///
    /// Returns error if neither Docker nor Podman are available or connection fails.
    pub async fn new() -> Result<Self> {
        Self::with_config(ContainerClientConfig::default()).await
    }

    /// Create a new container client with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns error if connection to container runtime fails.
    pub async fn with_config(config: ContainerClientConfig) -> Result<Self> {
        let docker = Self::connect().await?;

        let client = Self {
            docker: Arc::new(docker),
            config,
        };

        // Verify connection works
        client.ping().await?;

        Ok(client)
    }

    /// Connect to Docker or Podman daemon.
    ///
    /// Tries multiple connection strategies in order:
    /// 1. Local defaults (Unix socket or Windows named pipe)
    /// 2. DOCKER_HOST environment variable
    /// 3. Podman socket (if Docker fails)
    async fn connect() -> Result<Docker> {
        // Try local defaults first
        debug!("Attempting to connect to container runtime...");

        match Docker::connect_with_local_defaults() {
            Ok(docker) => {
                info!("Connected to container runtime via local defaults");
                return Ok(docker);
            }
            Err(e) => {
                debug!("Local defaults failed: {}", e);
            }
        }

        // Try Unix socket for Podman
        #[cfg(unix)]
        {
            use bollard::Docker;

            // Try rootless Podman socket
            if let Ok(home) = std::env::var("HOME") {
                let podman_socket = format!("unix://{}/run/podman/podman.sock", home);
                debug!("Trying Podman socket: {}", podman_socket);

                match Docker::connect_with_socket(&podman_socket, 120, bollard::API_DEFAULT_VERSION)
                {
                    Ok(docker) => {
                        info!("Connected to Podman via rootless socket");
                        return Ok(docker);
                    }
                    Err(e) => {
                        debug!("Podman rootless socket failed: {}", e);
                    }
                }
            }

            // Try system Podman socket
            let system_socket = "unix:///run/podman/podman.sock";
            debug!("Trying system Podman socket: {}", system_socket);

            match Docker::connect_with_socket(system_socket, 120, bollard::API_DEFAULT_VERSION) {
                Ok(docker) => {
                    info!("Connected to Podman via system socket");
                    return Ok(docker);
                }
                Err(e) => {
                    debug!("Podman system socket failed: {}", e);
                }
            }
        }

        Err(ContainerError::Other(
            "Failed to connect to Docker or Podman. Please ensure Docker or Podman is installed and running.".to_string()
        ))
    }

    /// Ping the container runtime to verify connectivity.
    ///
    /// # Errors
    ///
    /// Returns error if ping fails.
    pub async fn ping(&self) -> Result<()> {
        self.docker.ping().await.map_err(|e| {
            ContainerError::Other(format!("Failed to ping container runtime: {}", e))
        })?;
        debug!("Container runtime ping successful");
        Ok(())
    }

    /// Get version information from the container runtime.
    ///
    /// # Errors
    ///
    /// Returns error if version query fails.
    pub async fn version(&self) -> Result<bollard::models::SystemVersion> {
        self.docker
            .version()
            .await
            .map_err(|e| ContainerError::Other(format!("Failed to get version: {}", e)))
    }

    /// Get system information from the container runtime.
    ///
    /// # Errors
    ///
    /// Returns error if system info query fails.
    pub async fn info(&self) -> Result<bollard::models::SystemInfo> {
        self.docker
            .info()
            .await
            .map_err(|e| ContainerError::Other(format!("Failed to get info: {}", e)))
    }

    /// Get the underlying Docker client.
    ///
    /// This provides direct access to the bollard Docker API for advanced operations.
    pub fn docker(&self) -> &Docker {
        &self.docker
    }

    /// Check if the runtime is Docker or Podman.
    ///
    /// # Errors
    ///
    /// Returns error if runtime detection fails.
    pub async fn runtime_type(&self) -> Result<RuntimeType> {
        let version = self.version().await?;

        if version
            .components
            .and_then(|comps| {
                comps
                    .iter()
                    .find(|c| c.name == "Engine")
                    .map(|c| c.version.clone())
            })
            .filter(|name| name.to_lowercase().contains("podman"))
            .is_some()
        {
            return Ok(RuntimeType::Podman);
        }

        Ok(RuntimeType::Docker)
    }

    /// Check if an image exists locally.
    ///
    /// # Errors
    ///
    /// Returns error if image inspection fails.
    pub async fn image_exists(&self, image: &str) -> Result<bool> {
        match self.docker.inspect_image(image).await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(ContainerError::ApiError(e)),
        }
    }

    /// Get container ID by name.
    ///
    /// # Errors
    ///
    /// Returns error if container is not found or inspection fails.
    pub async fn get_container_id(&self, name: &str) -> Result<String> {
        let inspect = self
            .docker
            .inspect_container(
                name,
                None::<bollard::query_parameters::InspectContainerOptions>,
            )
            .await
            .map_err(|e| match e {
                bollard::errors::Error::DockerResponseServerError {
                    status_code: 404, ..
                } => ContainerError::NotFound(name.to_string()),
                e => ContainerError::ApiError(e),
            })?;

        inspect
            .id
            .ok_or_else(|| ContainerError::Other(format!("Container {} has no ID", name)))
    }

    /// Check if a container exists by name.
    ///
    /// # Errors
    ///
    /// Returns error if inspection fails for reasons other than not found.
    pub async fn container_exists(&self, name: &str) -> Result<bool> {
        match self.get_container_id(name).await {
            Ok(_) => Ok(true),
            Err(ContainerError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get container state (running, stopped, etc.) by name or ID.
    ///
    /// # Errors
    ///
    /// Returns error if container is not found or inspection fails.
    pub async fn container_state(&self, name_or_id: &str) -> Result<ContainerState> {
        let inspect = self
            .docker
            .inspect_container(
                name_or_id,
                None::<bollard::query_parameters::InspectContainerOptions>,
            )
            .await
            .map_err(|e| match e {
                bollard::errors::Error::DockerResponseServerError {
                    status_code: 404, ..
                } => ContainerError::NotFound(name_or_id.to_string()),
                e => ContainerError::ApiError(e),
            })?;

        let state = inspect.state.ok_or_else(|| {
            ContainerError::Other(format!("Container {} has no state", name_or_id))
        })?;

        if state.running.unwrap_or(false) {
            Ok(ContainerState::Running)
        } else if state.paused.unwrap_or(false) {
            Ok(ContainerState::Paused)
        } else if state.restarting.unwrap_or(false) {
            Ok(ContainerState::Restarting)
        } else if state.dead.unwrap_or(false) {
            Ok(ContainerState::Dead)
        } else {
            Ok(ContainerState::Stopped)
        }
    }
}

/// Container state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    /// Container is running
    Running,
    /// Container is paused
    Paused,
    /// Container is restarting
    Restarting,
    /// Container is stopped
    Stopped,
    /// Container is dead
    Dead,
}

/// Type of container runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeType {
    /// Docker runtime
    Docker,
    /// Podman runtime
    Podman,
}

impl std::fmt::Display for RuntimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeType::Docker => write!(f, "Docker"),
            RuntimeType::Podman => write!(f, "Podman"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Docker/Podman to be running
    async fn test_client_connection() {
        let client = ContainerClient::new().await.unwrap();
        client.ping().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_runtime_detection() {
        let client = ContainerClient::new().await.unwrap();
        let runtime_type = client.runtime_type().await.unwrap();
        println!("Runtime type: {}", runtime_type);
    }

    #[tokio::test]
    #[ignore]
    async fn test_version_info() {
        let client = ContainerClient::new().await.unwrap();
        let version = client.version().await.unwrap();
        println!("Version: {:?}", version);
    }
}
