//! Container networking management.
//!
//! Provides APIs for creating and managing container networks with isolation.

use crate::container::{ContainerError, Result};
use bollard::Docker;
use std::collections::HashMap;
use tracing::{debug, info};

/// Network configuration.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Network name
    pub name: String,
    /// Network driver (bridge, host, none, overlay)
    pub driver: String,
    /// Enable IPv6
    pub enable_ipv6: bool,
    /// Internal network (no external connectivity)
    pub internal: bool,
    /// Network labels
    pub labels: HashMap<String, String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            name: "aca-network".to_string(),
            driver: "bridge".to_string(),
            enable_ipv6: false,
            internal: false,
            labels: HashMap::new(),
        }
    }
}

/// Network manager for container networking.
pub struct NetworkManager {
    docker: Docker,
}

impl NetworkManager {
    /// Create a new network manager.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Create a new network.
    ///
    /// # Errors
    ///
    /// Returns error if network creation fails.
    pub async fn create_network(&self, config: &NetworkConfig) -> Result<String> {
        debug!("Creating network: {}", config.name);

        let labels: HashMap<&str, &str> = config
            .labels
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let response = self
            .docker
            .create_network(bollard::network::CreateNetworkOptions {
                name: config.name.as_str(),
                driver: config.driver.as_str(),
                enable_ipv6: config.enable_ipv6,
                internal: config.internal,
                labels,
                ..Default::default()
            })
            .await?;

        info!("Created network: {} ({})", config.name, response.id);

        Ok(response.id)
    }

    /// Remove a network.
    ///
    /// # Errors
    ///
    /// Returns error if network removal fails.
    pub async fn remove_network(&self, network_id: &str) -> Result<()> {
        debug!("Removing network: {}", network_id);
        self.docker.remove_network(network_id).await?;
        info!("Removed network: {}", network_id);
        Ok(())
    }

    /// List all networks.
    ///
    /// # Errors
    ///
    /// Returns error if listing fails.
    pub async fn list_networks(&self) -> Result<Vec<NetworkInfo>> {
        let networks = self
            .docker
            .list_networks(None::<bollard::network::ListNetworksOptions<String>>)
            .await?;

        Ok(networks
            .into_iter()
            .map(|n| NetworkInfo {
                id: n.id.unwrap_or_default(),
                name: n.name.unwrap_or_default(),
                driver: n.driver.unwrap_or_default(),
            })
            .collect())
    }

    /// Check if a network exists.
    ///
    /// # Errors
    ///
    /// Returns error if network inspection fails.
    pub async fn network_exists(&self, network_name: &str) -> Result<bool> {
        match self
            .docker
            .inspect_network(
                network_name,
                None::<bollard::network::InspectNetworkOptions<String>>,
            )
            .await
        {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(ContainerError::ApiError(e)),
        }
    }
}

/// Network information.
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    /// Network ID
    pub id: String,
    /// Network name
    pub name: String,
    /// Network driver
    pub driver: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.driver, "bridge");
        assert!(!config.internal);
    }

    #[tokio::test]
    #[ignore] // Requires Docker/Podman
    async fn test_network_lifecycle() {
        use bollard::Docker;

        let docker = Docker::connect_with_local_defaults().unwrap();
        let manager = NetworkManager::new(docker);

        let config = NetworkConfig {
            name: format!("test-network-{}", uuid::Uuid::new_v4()),
            driver: "bridge".to_string(),
            ..Default::default()
        };

        // Create network
        let network_id = manager.create_network(&config).await.unwrap();

        // Verify it exists
        assert!(manager.network_exists(&config.name).await.unwrap());

        // Remove network
        manager.remove_network(&network_id).await.unwrap();
    }
}
