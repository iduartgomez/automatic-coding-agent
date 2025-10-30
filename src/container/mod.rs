//! Container orchestration and isolation layer.
//!
//! This module provides containerized execution environments for task isolation
//! using Docker/Podman via the bollard API. It handles container lifecycle management,
//! resource limits, networking, volume mounting, and monitoring.
//!
//! ## Architecture
//!
//! The container module is organized into several components:
//!
//! - [`client`]: Docker/Podman API client wrapper with connection management
//! - [`orchestrator`]: High-level container lifecycle orchestration
//! - [`config`]: Container configuration builders for programmatic setup
//! - [`executor`]: Command execution within running containers
//! - [`monitor`]: Resource monitoring and health tracking
//! - [`network`]: Network isolation and management
//! - [`volume`]: Volume mounting and lifecycle management
//!
//! ## Usage
//!
//! ```rust,no_run
//! use aca::container::{ContainerOrchestrator, ContainerConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create orchestrator
//!     let orchestrator = ContainerOrchestrator::new().await?;
//!
//!     // Configure container
//!     let config = ContainerConfig::builder()
//!         .image("ubuntu:22.04")
//!         .workspace("/workspace")
//!         .memory_limit(2_147_483_648) // 2GB
//!         .cpu_quota(100000)
//!         .build()?;
//!
//!     // Create and start container
//!     let container_id = orchestrator.create_container(&config).await?;
//!     orchestrator.start_container(&container_id).await?;
//!
//!     // Execute command
//!     let output = orchestrator.exec(&container_id, vec!["echo", "Hello"]).await?;
//!     println!("{}", output);
//!
//!     // Cleanup
//!     orchestrator.stop_and_remove(&container_id).await?;
//!     Ok(())
//! }
//! ```

mod client;
mod config;
mod executor;
mod image;
mod interactive;
mod monitor;
mod network;
mod orchestrator;
mod volume;

pub use client::{ContainerClient, ContainerClientConfig, RuntimeType};
pub use config::{ContainerConfig, ContainerConfigBuilder};
pub use executor::{ExecConfig, ExecOutput};
pub use image::{ImageBuilder, ImageInfo, ACA_BASE_IMAGE, ACA_BASE_IMAGE_ALPINE};
pub use interactive::{InteractiveSession, attach_to_container};
pub use monitor::{ContainerStats, ResourceMonitor};
pub use network::{NetworkConfig, NetworkManager};
pub use orchestrator::{ContainerOrchestrator, ContainerOrchestratorConfig};
pub use volume::{VolumeConfig, VolumeManager};

/// Container runtime errors.
#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    /// Docker/Podman API error
    #[error("Container API error: {0}")]
    ApiError(#[from] bollard::errors::Error),

    /// Container not found
    #[error("Container not found: {0}")]
    NotFound(String),

    /// Container configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Container execution error
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Volume error
    #[error("Volume error: {0}")]
    VolumeError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// General error
    #[error("Container error: {0}")]
    Other(String),
}

/// Result type for container operations.
pub type Result<T> = std::result::Result<T, ContainerError>;
