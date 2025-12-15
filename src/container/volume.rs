//! Container volume management.
//!
//! Provides APIs for creating and managing container volumes for persistent storage.

use crate::container::{ContainerError, Result};
use bollard::Docker;
use std::collections::HashMap;
use tracing::{debug, info};

/// Volume configuration.
#[derive(Debug, Clone)]
pub struct VolumeConfig {
    /// Volume name
    pub name: String,
    /// Volume driver
    pub driver: String,
    /// Driver options
    pub driver_opts: HashMap<String, String>,
    /// Volume labels
    pub labels: HashMap<String, String>,
}

impl Default for VolumeConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            driver: "local".to_string(),
            driver_opts: HashMap::new(),
            labels: HashMap::new(),
        }
    }
}

/// Volume manager for persistent storage.
pub struct VolumeManager {
    docker: Docker,
}

impl VolumeManager {
    /// Create a new volume manager.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Create a new volume.
    ///
    /// # Errors
    ///
    /// Returns error if volume creation fails.
    pub async fn create_volume(&self, config: &VolumeConfig) -> Result<String> {
        debug!("Creating volume: {}", config.name);

        let driver_opts: HashMap<&str, &str> = config
            .driver_opts
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let labels: HashMap<&str, &str> = config
            .labels
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let response = self
            .docker
            .create_volume(bollard::volume::CreateVolumeOptions {
                name: config.name.as_str(),
                driver: config.driver.as_str(),
                driver_opts,
                labels,
            })
            .await?;

        info!("Created volume: {}", response.name);

        Ok(response.name)
    }

    /// Remove a volume.
    ///
    /// # Errors
    ///
    /// Returns error if volume removal fails.
    pub async fn remove_volume(&self, volume_name: &str, force: bool) -> Result<()> {
        debug!("Removing volume: {}", volume_name);

        self.docker
            .remove_volume(
                volume_name,
                Some(bollard::volume::RemoveVolumeOptions { force }),
            )
            .await?;

        info!("Removed volume: {}", volume_name);
        Ok(())
    }

    /// List all volumes.
    ///
    /// # Errors
    ///
    /// Returns error if listing fails.
    pub async fn list_volumes(&self) -> Result<Vec<VolumeInfo>> {
        let response = self
            .docker
            .list_volumes(None::<bollard::volume::ListVolumesOptions<String>>)
            .await?;

        Ok(response
            .volumes
            .unwrap_or_default()
            .into_iter()
            .map(|v| VolumeInfo {
                name: v.name,
                driver: v.driver,
                mountpoint: v.mountpoint,
            })
            .collect())
    }

    /// Check if a volume exists.
    ///
    /// # Errors
    ///
    /// Returns error if volume inspection fails.
    pub async fn volume_exists(&self, volume_name: &str) -> Result<bool> {
        match self.docker.inspect_volume(volume_name).await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(ContainerError::ApiError(e)),
        }
    }
}

/// Volume information.
#[derive(Debug, Clone)]
pub struct VolumeInfo {
    /// Volume name
    pub name: String,
    /// Volume driver
    pub driver: String,
    /// Mount point on host
    pub mountpoint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_config_default() {
        let config = VolumeConfig::default();
        assert_eq!(config.driver, "local");
        assert!(config.driver_opts.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires Docker/Podman
    async fn test_volume_lifecycle() {
        use bollard::Docker;

        let docker = Docker::connect_with_local_defaults().unwrap();
        let manager = VolumeManager::new(docker);

        let config = VolumeConfig {
            name: format!("test-volume-{}", uuid::Uuid::new_v4()),
            ..Default::default()
        };

        // Create volume
        let volume_name = manager.create_volume(&config).await.unwrap();

        // Verify it exists
        assert!(manager.volume_exists(&volume_name).await.unwrap());

        // Remove volume
        manager.remove_volume(&volume_name, true).await.unwrap();
    }
}
