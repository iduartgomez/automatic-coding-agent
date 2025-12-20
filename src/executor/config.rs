//! Execution mode configuration types.
//!
//! Defines configuration structures for different execution modes
//! (host vs container).

use serde::{Deserialize, Serialize};

/// Execution mode - where commands run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Execute commands directly on host (default)
    Host,

    /// Execute commands inside a container
    Container(ContainerExecutionConfig),
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Host
    }
}

impl ExecutionMode {
    /// Check if this is container mode
    pub fn is_container(&self) -> bool {
        matches!(self, ExecutionMode::Container(_))
    }

    /// Check if this is host mode
    pub fn is_host(&self) -> bool {
        matches!(self, ExecutionMode::Host)
    }
}

/// Container execution configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerExecutionConfig {
    /// Container image to use (default: "alpine:latest")
    #[serde(default = "default_image")]
    pub image: String,

    /// Resource allocation percentage (0.0-1.0, default: 0.5)
    #[serde(default = "default_resource_percentage")]
    pub resource_percentage: f64,

    /// Override memory limit in bytes (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit_bytes: Option<i64>,

    /// Override CPU quota (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_quota: Option<i64>,
}

fn default_image() -> String {
    "alpine:latest".to_string()
}

fn default_resource_percentage() -> f64 {
    0.5
}

impl Default for ContainerExecutionConfig {
    fn default() -> Self {
        Self {
            image: default_image(),
            resource_percentage: default_resource_percentage(),
            memory_limit_bytes: None,
            cpu_quota: None,
        }
    }
}

impl ContainerExecutionConfig {
    /// Create a new container execution config with the specified image
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            ..Default::default()
        }
    }

    /// Set resource allocation percentage (0.0 to 1.0)
    pub fn with_resource_percentage(mut self, percentage: f64) -> Self {
        self.resource_percentage = percentage.clamp(0.0, 1.0);
        self
    }

    /// Set memory limit in bytes
    pub fn with_memory_limit(mut self, bytes: i64) -> Self {
        self.memory_limit_bytes = Some(bytes);
        self
    }

    /// Set CPU quota
    pub fn with_cpu_quota(mut self, quota: i64) -> Self {
        self.cpu_quota = Some(quota);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_default() {
        let mode = ExecutionMode::default();
        assert!(mode.is_host());
        assert!(!mode.is_container());
    }

    #[test]
    fn test_container_config_default() {
        let config = ContainerExecutionConfig::default();
        assert_eq!(config.image, "alpine:latest");
        assert_eq!(config.resource_percentage, 0.5);
        assert!(config.memory_limit_bytes.is_none());
        assert!(config.cpu_quota.is_none());
    }

    #[test]
    fn test_container_config_builder() {
        let config = ContainerExecutionConfig::new("ubuntu:22.04")
            .with_resource_percentage(0.75)
            .with_memory_limit(1_073_741_824)
            .with_cpu_quota(100_000);

        assert_eq!(config.image, "ubuntu:22.04");
        assert_eq!(config.resource_percentage, 0.75);
        assert_eq!(config.memory_limit_bytes, Some(1_073_741_824));
        assert_eq!(config.cpu_quota, Some(100_000));
    }

    #[test]
    fn test_resource_percentage_clamping() {
        let config = ContainerExecutionConfig::default().with_resource_percentage(1.5);
        assert_eq!(config.resource_percentage, 1.0);

        let config = ContainerExecutionConfig::default().with_resource_percentage(-0.5);
        assert_eq!(config.resource_percentage, 0.0);
    }
}
