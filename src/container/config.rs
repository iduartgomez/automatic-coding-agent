//! Container configuration builders.
//!
//! Provides a fluent API for building container configurations programmatically
//! without manual Dockerfiles or config files.

use crate::container::{ContainerError, Result};
use bollard::service::{HostConfig, Mount, MountTypeEnum, PortBinding};
use std::collections::HashMap;

/// Container configuration builder.
///
/// Provides a fluent interface for constructing container configurations
/// with sane defaults and validation.
pub struct ContainerConfigBuilder {
    image: Option<String>,
    cmd: Option<Vec<String>>,
    entrypoint: Option<Vec<String>>,
    working_dir: Option<String>,
    env: Vec<String>,
    labels: HashMap<String, String>,
    memory_limit: Option<i64>,
    memory_swap: Option<i64>,
    cpu_quota: Option<i64>,
    cpu_period: Option<i64>,
    binds: Vec<String>,
    mounts: Vec<Mount>,
    network_mode: Option<String>,
    port_bindings: HashMap<String, Option<Vec<PortBinding>>>,
    auto_remove: bool,
    privileged: bool,
    readonly_rootfs: bool,
    user: Option<String>,
}

impl Default for ContainerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerConfigBuilder {
    /// Create a new container configuration builder.
    pub fn new() -> Self {
        Self {
            image: None,
            cmd: None,
            entrypoint: None,
            working_dir: None,
            env: Vec::new(),
            labels: HashMap::new(),
            memory_limit: None,
            memory_swap: None,
            cpu_quota: None,
            cpu_period: None,
            binds: Vec::new(),
            mounts: Vec::new(),
            network_mode: None,
            port_bindings: HashMap::new(),
            auto_remove: false,
            privileged: false,
            readonly_rootfs: false,
            user: None,
        }
    }

    /// Set the container image.
    pub fn image<S: Into<String>>(mut self, image: S) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Set the command to run in the container.
    pub fn cmd<I, S>(mut self, cmd: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.cmd = Some(cmd.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Set the entrypoint for the container.
    pub fn entrypoint<I, S>(mut self, entrypoint: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.entrypoint = Some(entrypoint.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Set the working directory in the container.
    pub fn working_dir<S: Into<String>>(mut self, dir: S) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add an environment variable.
    pub fn env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env.push(format!("{}={}", key.into(), value.into()));
        self
    }

    /// Add multiple environment variables.
    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in envs {
            self.env.push(format!("{}={}", k.into(), v.into()));
        }
        self
    }

    /// Add a label to the container.
    pub fn label<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Set memory limit in bytes.
    pub fn memory_limit(mut self, bytes: i64) -> Self {
        self.memory_limit = Some(bytes);
        self
    }

    /// Set memory + swap limit in bytes.
    pub fn memory_swap(mut self, bytes: i64) -> Self {
        self.memory_swap = Some(bytes);
        self
    }

    /// Set CPU quota in microseconds per period.
    pub fn cpu_quota(mut self, quota: i64) -> Self {
        self.cpu_quota = Some(quota);
        self
    }

    /// Set CPU period in microseconds (default 100000).
    pub fn cpu_period(mut self, period: i64) -> Self {
        self.cpu_period = Some(period);
        self
    }

    /// Add a volume bind mount (host_path:container_path[:mode]).
    pub fn bind<S: Into<String>>(mut self, bind: S) -> Self {
        self.binds.push(bind.into());
        self
    }

    /// Add a volume mount with full control.
    pub fn mount(mut self, source: String, target: String, read_only: bool) -> Self {
        self.mounts.push(Mount {
            target: Some(target),
            source: Some(source),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(read_only),
            ..Default::default()
        });
        self
    }

    /// Set network mode (e.g., "bridge", "host", "none").
    pub fn network_mode<S: Into<String>>(mut self, mode: S) -> Self {
        self.network_mode = Some(mode.into());
        self
    }

    /// Add a port binding (container_port/protocol -> host_port).
    pub fn port_binding<S: Into<String>>(mut self, container_port: S, host_port: u16) -> Self {
        self.port_bindings.insert(
            container_port.into(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(host_port.to_string()),
            }]),
        );
        self
    }

    /// Enable auto-removal of container on exit.
    pub fn auto_remove(mut self, enable: bool) -> Self {
        self.auto_remove = enable;
        self
    }

    /// Run container in privileged mode.
    pub fn privileged(mut self, enable: bool) -> Self {
        self.privileged = enable;
        self
    }

    /// Make root filesystem read-only.
    pub fn readonly_rootfs(mut self, enable: bool) -> Self {
        self.readonly_rootfs = enable;
        self
    }

    /// Set user to run as in the container.
    pub fn user<S: Into<String>>(mut self, user: S) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Build the container configuration.
    ///
    /// # Errors
    ///
    /// Returns error if required fields are missing or invalid.
    pub fn build(self) -> Result<ContainerConfig> {
        let image = self
            .image
            .ok_or_else(|| ContainerError::ConfigError("Image is required".to_string()))?;

        let host_config = HostConfig {
            binds: if self.binds.is_empty() {
                None
            } else {
                Some(self.binds.clone())
            },
            mounts: if self.mounts.is_empty() {
                None
            } else {
                Some(self.mounts.clone())
            },
            memory: self.memory_limit,
            memory_swap: self.memory_swap,
            cpu_quota: self.cpu_quota,
            cpu_period: self.cpu_period,
            network_mode: self.network_mode.clone(),
            port_bindings: if self.port_bindings.is_empty() {
                None
            } else {
                Some(self.port_bindings.clone())
            },
            auto_remove: Some(self.auto_remove),
            privileged: Some(self.privileged),
            readonly_rootfs: Some(self.readonly_rootfs),
            ..Default::default()
        };

        Ok(ContainerConfig {
            image,
            cmd: self.cmd,
            entrypoint: self.entrypoint,
            working_dir: self.working_dir,
            env: if self.env.is_empty() {
                None
            } else {
                Some(self.env)
            },
            labels: if self.labels.is_empty() {
                None
            } else {
                Some(self.labels)
            },
            user: self.user,
            host_config,
        })
    }
}

/// Container configuration.
///
/// Holds container configuration for creation.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Image name
    pub image: String,
    /// Command to run
    pub cmd: Option<Vec<String>>,
    /// Entrypoint
    pub entrypoint: Option<Vec<String>>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env: Option<Vec<String>>,
    /// Labels
    pub labels: Option<HashMap<String, String>>,
    /// User
    pub user: Option<String>,
    /// Host configuration
    pub host_config: HostConfig,
}

impl ContainerConfig {
    /// Create a new configuration builder.
    pub fn builder() -> ContainerConfigBuilder {
        ContainerConfigBuilder::new()
    }

    /// Get the image name.
    pub fn image(&self) -> Option<&str> {
        Some(&self.image)
    }

    /// Get the working directory.
    pub fn working_dir(&self) -> Option<&str> {
        self.working_dir.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_config() {
        let config = ContainerConfig::builder()
            .image("ubuntu:22.04")
            .cmd(vec!["echo", "hello"])
            .working_dir("/workspace")
            .build()
            .unwrap();

        assert_eq!(config.image(), Some("ubuntu:22.04"));
        assert_eq!(config.working_dir(), Some("/workspace"));
    }

    #[test]
    fn test_resource_limits() {
        let config = ContainerConfig::builder()
            .image("ubuntu:22.04")
            .memory_limit(2_147_483_648) // 2GB
            .cpu_quota(100000)
            .build()
            .unwrap();

        assert_eq!(config.host_config.memory, Some(2_147_483_648));
        assert_eq!(config.host_config.cpu_quota, Some(100000));
    }

    #[test]
    fn test_environment_variables() {
        let config = ContainerConfig::builder()
            .image("ubuntu:22.04")
            .env("FOO", "bar")
            .env("BAZ", "qux")
            .build()
            .unwrap();

        let env = config.env.unwrap();
        assert!(env.contains(&"FOO=bar".to_string()));
        assert!(env.contains(&"BAZ=qux".to_string()));
    }

    #[test]
    fn test_volume_binds() {
        let config = ContainerConfig::builder()
            .image("ubuntu:22.04")
            .bind("/host/path:/container/path:ro")
            .build()
            .unwrap();

        let binds = config.host_config.binds.unwrap();
        assert_eq!(binds.len(), 1);
        assert_eq!(binds[0], "/host/path:/container/path:ro");
    }

    #[test]
    fn test_missing_image_error() {
        let result = ContainerConfig::builder().cmd(vec!["echo"]).build();

        assert!(result.is_err());
        assert!(matches!(result, Err(ContainerError::ConfigError(_))));
    }
}
