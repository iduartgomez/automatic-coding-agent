//! Session-to-container lifecycle binding.
//!
//! This module provides lifecycle management that binds Docker/Podman containers
//! to session lifecycles. It ensures containers are properly created when sessions
//! start and cleaned up when sessions end.

use super::{ContainerConfig, ContainerError, ContainerOrchestrator, Result};
use crate::session::metadata::{
    ContainerResourceLimits, ContainerStatus, SessionContainerInfo, SessionId,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Configuration for session-bound container lifecycle management.
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Container image to use
    pub image: String,
    /// Workspace path to mount (host path)
    pub workspace_path: PathBuf,
    /// ACA directory path to mount (host path)
    pub aca_path: PathBuf,
    /// Memory limit in bytes
    pub memory_bytes: Option<i64>,
    /// CPU quota (microseconds per period)
    pub cpu_quota: Option<i64>,
    /// Whether to auto-remove container on shutdown
    pub auto_remove: bool,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            image: crate::executor::config::DEFAULT_CONTAINER_IMAGE.to_string(),
            workspace_path: PathBuf::new(),
            aca_path: PathBuf::new(),
            memory_bytes: None,
            cpu_quota: None,
            auto_remove: true,
        }
    }
}

/// Manages the lifecycle binding between sessions and containers.
///
/// This manager ensures that:
/// - Containers are named with session IDs for easy identification
/// - Container state is tracked in session metadata
/// - Containers are properly cleaned up when sessions end
/// - Existing containers can be reconnected on session restore
pub struct ContainerLifecycleManager {
    orchestrator: Arc<ContainerOrchestrator>,
    config: LifecycleConfig,
    /// Current session ID this manager is bound to
    session_id: SessionId,
    /// Current container ID (if container is running)
    container_id: Arc<RwLock<Option<String>>>,
    /// Container info for session metadata updates
    container_info: Arc<RwLock<Option<SessionContainerInfo>>>,
}

impl ContainerLifecycleManager {
    /// Create a new lifecycle manager bound to a session.
    pub async fn new(session_id: SessionId, config: LifecycleConfig) -> Result<Self> {
        let orchestrator = ContainerOrchestrator::new().await?;

        Ok(Self {
            orchestrator: Arc::new(orchestrator),
            config,
            session_id,
            container_id: Arc::new(RwLock::new(None)),
            container_info: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a new lifecycle manager with an existing orchestrator.
    pub fn with_orchestrator(
        session_id: SessionId,
        orchestrator: Arc<ContainerOrchestrator>,
        config: LifecycleConfig,
    ) -> Self {
        Self {
            orchestrator,
            config,
            session_id,
            container_id: Arc::new(RwLock::new(None)),
            container_info: Arc::new(RwLock::new(None)),
        }
    }

    /// Generate the container name for a session.
    pub fn container_name_for_session(session_id: &SessionId) -> String {
        // Use first 12 chars of session UUID for shorter names
        let short_id = session_id.to_string();
        let short_id = short_id.get(..12).unwrap_or(&short_id);
        format!("aca-session-{}", short_id)
    }

    /// Get the container name for this session.
    pub fn container_name(&self) -> String {
        Self::container_name_for_session(&self.session_id)
    }

    /// Get the session ID this manager is bound to.
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Get the current container ID if one exists.
    pub async fn current_container_id(&self) -> Option<String> {
        self.container_id.read().await.clone()
    }

    /// Get the current container info.
    pub async fn container_info(&self) -> Option<SessionContainerInfo> {
        self.container_info.read().await.clone()
    }

    /// Start a container for this session.
    ///
    /// Creates and starts a new container bound to this session. The container
    /// is named with the session ID and labeled for easy identification.
    pub async fn start_session_container(&self) -> Result<SessionContainerInfo> {
        let container_name = self.container_name();

        info!(
            "Starting session container '{}' for session {}",
            container_name, self.session_id
        );

        // Check if container already exists
        if let Some(existing_id) = self.try_find_existing_container().await? {
            info!("Found existing container {} for session", existing_id);

            // Try to start it if stopped
            if let Err(e) = self.orchestrator.start_container(&existing_id).await {
                debug!("Container may already be running: {}", e);
            }

            return self.create_container_info(existing_id).await;
        }

        // Build container configuration
        let mut container_config = ContainerConfig::builder()
            .image(&self.config.image)
            .cmd(vec!["sleep", "infinity"])
            .working_dir("/workspace")
            .label("aca.session.id", self.session_id.to_string())
            .label("aca.managed", "true");

        // Mount workspace
        if !self.config.workspace_path.as_os_str().is_empty() {
            container_config = container_config.bind(format!(
                "{}:/workspace:rw",
                self.config.workspace_path.display()
            ));
        }

        // Mount .aca directory
        if !self.config.aca_path.as_os_str().is_empty() {
            container_config =
                container_config.bind(format!("{}:/.aca:rw", self.config.aca_path.display()));
        }

        // Apply resource limits
        if let Some(mem) = self.config.memory_bytes {
            container_config = container_config.memory_limit(mem);
        }
        if let Some(cpu) = self.config.cpu_quota {
            container_config = container_config.cpu_quota(cpu);
        }

        let container_config = container_config
            .build()
            .map_err(|e| ContainerError::ConfigError(e.to_string()))?;

        // Create and start container
        let container_id = self
            .orchestrator
            .create_container(&container_config, Some(&container_name))
            .await?;

        self.orchestrator.start_container(&container_id).await?;

        info!(
            "Session container started: {} ({})",
            container_name,
            container_id.get(..12).unwrap_or(&container_id)
        );

        self.create_container_info(container_id).await
    }

    /// Create container info and update internal state.
    async fn create_container_info(&self, container_id: String) -> Result<SessionContainerInfo> {
        let container_info = SessionContainerInfo::new(
            container_id.clone(),
            self.container_name(),
            self.config.image.clone(),
            Some(ContainerResourceLimits {
                memory_bytes: self.config.memory_bytes,
                cpu_quota: self.config.cpu_quota,
            }),
        );

        // Update internal state
        *self.container_id.write().await = Some(container_id);
        *self.container_info.write().await = Some(container_info.clone());

        Ok(container_info)
    }

    /// Try to find an existing container for this session.
    async fn try_find_existing_container(&self) -> Result<Option<String>> {
        let container_name = self.container_name();

        // Try to get container by name
        match self
            .orchestrator
            .client()
            .get_container_id(&container_name)
            .await
        {
            Ok(id) => Ok(Some(id)),
            Err(_) => Ok(None),
        }
    }

    /// Ensure a container is running for this session.
    ///
    /// If no container exists, creates one. If a container exists but is stopped,
    /// attempts to start it. Returns the container ID.
    pub async fn ensure_container(&self) -> Result<String> {
        // Check if we already have a running container
        if let Some(id) = self.current_container_id().await {
            return Ok(id);
        }

        // Start a new session container
        let info = self.start_session_container().await?;
        Ok(info.container_id)
    }

    /// Stop the session container.
    ///
    /// Gracefully stops the container without removing it.
    pub async fn stop_session_container(&self) -> Result<()> {
        let id_guard = self.container_id.read().await;

        if let Some(ref id) = *id_guard {
            info!("Stopping session container: {}", self.container_name());
            self.orchestrator.stop_container(id).await?;

            // Update container info status
            if let Some(ref mut info) = *self.container_info.write().await {
                info.mark_stopped();
            }
        }

        Ok(())
    }

    /// Stop and remove the session container.
    ///
    /// This should be called when the session is shutting down.
    pub async fn shutdown(&self) -> Result<()> {
        let mut id_guard = self.container_id.write().await;

        if let Some(ref id) = *id_guard {
            info!(
                "Shutting down session container: {} ({})",
                self.container_name(),
                id.get(..12).unwrap_or(id)
            );

            if self.config.auto_remove {
                self.orchestrator.stop_and_remove(id).await?;
            } else {
                self.orchestrator.stop_container(id).await?;
            }

            // Update container info status
            if let Some(ref mut info) = *self.container_info.write().await {
                if self.config.auto_remove {
                    info.mark_removed();
                } else {
                    info.mark_stopped();
                }
            }

            *id_guard = None;
        }

        Ok(())
    }

    /// Reconnect to an existing container from session metadata.
    ///
    /// Used when restoring a session that had a running container.
    pub async fn reconnect(&self, container_info: &SessionContainerInfo) -> Result<bool> {
        if container_info.status == ContainerStatus::Removed {
            debug!("Container was previously removed, cannot reconnect");
            return Ok(false);
        }

        // Try to find the container
        match self
            .orchestrator
            .client()
            .get_container_id(&container_info.container_name)
            .await
        {
            Ok(id) => {
                // Verify container ID matches
                if id != container_info.container_id {
                    warn!(
                        "Container ID mismatch: expected {}, found {}",
                        container_info.container_id, id
                    );
                }

                // Try to start if stopped
                if let Err(e) = self.orchestrator.start_container(&id).await {
                    debug!("Container may already be running: {}", e);
                }

                // Update internal state
                *self.container_id.write().await = Some(id.clone());
                *self.container_info.write().await = Some(SessionContainerInfo {
                    status: ContainerStatus::Running,
                    ..container_info.clone()
                });

                info!(
                    "Reconnected to session container: {}",
                    container_info.container_name
                );
                Ok(true)
            }
            Err(_) => {
                info!(
                    "Previous container {} not found, will create new one on demand",
                    container_info.container_name
                );
                Ok(false)
            }
        }
    }

    /// Get the orchestrator for direct container operations.
    pub fn orchestrator(&self) -> &Arc<ContainerOrchestrator> {
        &self.orchestrator
    }

    /// Check if the session container is currently running.
    pub async fn is_container_running(&self) -> bool {
        self.container_id.read().await.is_some()
    }

    /// Get container health status.
    pub async fn health_check(&self) -> Result<bool> {
        if let Some(ref id) = *self.container_id.read().await {
            // Execute a simple command to check container health
            match self.orchestrator.exec(id, vec!["true"]).await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_container_name_generation() {
        let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let name = ContainerLifecycleManager::container_name_for_session(&session_id);
        assert!(name.starts_with("aca-session-"));
        assert!(name.contains("550e8400-e2"));
    }

    #[test]
    fn test_container_name_uniqueness() {
        let session_id1 = Uuid::new_v4();
        let session_id2 = Uuid::new_v4();

        let name1 = ContainerLifecycleManager::container_name_for_session(&session_id1);
        let name2 = ContainerLifecycleManager::container_name_for_session(&session_id2);

        assert_ne!(
            name1, name2,
            "Different sessions should have different container names"
        );
    }

    #[test]
    fn test_lifecycle_config_default() {
        let config = LifecycleConfig::default();
        assert!(config.auto_remove);
        assert!(config.memory_bytes.is_none());
        assert!(config.cpu_quota.is_none());
        assert!(config.workspace_path.as_os_str().is_empty());
        assert!(config.aca_path.as_os_str().is_empty());
    }

    #[test]
    fn test_lifecycle_config_with_values() {
        let config = LifecycleConfig {
            image: "ubuntu:22.04".to_string(),
            workspace_path: PathBuf::from("/workspace"),
            aca_path: PathBuf::from("/workspace/.aca"),
            memory_bytes: Some(1024 * 1024 * 1024), // 1GB
            cpu_quota: Some(50000),
            auto_remove: false,
        };

        assert_eq!(config.image, "ubuntu:22.04");
        assert!(!config.auto_remove);
        assert_eq!(config.memory_bytes, Some(1024 * 1024 * 1024));
        assert_eq!(config.cpu_quota, Some(50000));
    }

    #[test]
    fn test_session_container_info_creation() {
        let info = SessionContainerInfo::new(
            "abc123def456".to_string(),
            "aca-session-test".to_string(),
            "alpine:latest".to_string(),
            Some(ContainerResourceLimits {
                memory_bytes: Some(512 * 1024 * 1024),
                cpu_quota: Some(25000),
            }),
        );

        assert_eq!(info.container_id, "abc123def456");
        assert_eq!(info.container_name, "aca-session-test");
        assert_eq!(info.image, "alpine:latest");
        assert!(info.is_running());
        assert_eq!(info.status, ContainerStatus::Running);
    }

    #[test]
    fn test_session_container_info_status_transitions() {
        let mut info = SessionContainerInfo::new(
            "abc123".to_string(),
            "test".to_string(),
            "alpine".to_string(),
            None,
        );

        assert!(info.is_running());

        info.mark_stopped();
        assert!(!info.is_running());
        assert_eq!(info.status, ContainerStatus::Stopped);

        info.mark_removed();
        assert!(!info.is_running());
        assert_eq!(info.status, ContainerStatus::Removed);
    }
}
