//! Container image building and management.
//!
//! Provides APIs for building custom Docker images programmatically,
//! including the ACA base image with Claude Code and development tools.

use crate::container::{ContainerError, Result};
use bollard::Docker;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

/// Default ACA base image name
pub const ACA_BASE_IMAGE: &str = "aca-dev:latest";

/// Image builder for creating custom container images.
pub struct ImageBuilder {
    docker: Docker,
}

impl ImageBuilder {
    /// Create a new image builder.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Build the ACA base image from a Dockerfile using Docker CLI.
    ///
    /// Note: This function uses the Docker CLI instead of the API due to
    /// complexity with tar streaming. Ensure docker/podman command is available.
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Path to the directory containing Dockerfile
    /// * `tag` - Tag for the built image (default: "aca-dev:latest")
    ///
    /// # Errors
    ///
    /// Returns error if Docker CLI is not available or build fails.
    pub async fn build_aca_base_image(
        &self,
        dockerfile_path: &Path,
        tag: Option<&str>,
    ) -> Result<String> {
        let image_tag = tag.unwrap_or(ACA_BASE_IMAGE);
        info!("Building ACA base image: {} using Docker CLI", image_tag);

        // Use Docker CLI for building
        use tokio::process::Command;

        let output = Command::new("docker")
            .arg("build")
            .arg("-t")
            .arg(image_tag)
            .arg("-f")
            .arg(dockerfile_path.join("Dockerfile"))
            .arg(dockerfile_path)
            .output()
            .await
            .map_err(|e| ContainerError::Other(format!("Failed to run docker build: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::Other(format!(
                "Docker build failed: {}",
                stderr
            )));
        }

        info!("Successfully built image: {}", image_tag);
        Ok(image_tag.to_string())
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
                status_code: 404,
                ..
            }) => Ok(false),
            Err(e) => Err(ContainerError::ApiError(e)),
        }
    }

    /// Pull an image from a registry.
    ///
    /// # Errors
    ///
    /// Returns error if image pull fails.
    pub async fn pull_image(&self, image: &str) -> Result<()> {
        info!("Pulling image: {}", image);

        let mut stream = self.docker.create_image(
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
                        debug!("Pull: {}", status);
                    }
                    if let Some(error) = info.error {
                        return Err(ContainerError::Other(format!("Pull failed: {}", error)));
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

    /// Remove an image.
    ///
    /// # Errors
    ///
    /// Returns error if image removal fails.
    pub async fn remove_image(&self, image: &str, force: bool) -> Result<()> {
        info!("Removing image: {}", image);

        self.docker
            .remove_image(
                image,
                Some(bollard::image::RemoveImageOptions {
                    force,
                    ..Default::default()
                }),
                None,
            )
            .await?;

        info!("Successfully removed image: {}", image);
        Ok(())
    }

    /// List all images.
    ///
    /// # Errors
    ///
    /// Returns error if listing fails.
    pub async fn list_images(&self) -> Result<Vec<ImageInfo>> {
        let images = self
            .docker
            .list_images(Some(bollard::image::ListImagesOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await?;

        Ok(images
            .into_iter()
            .map(|img| ImageInfo {
                id: img.id,
                repo_tags: img.repo_tags,
                size: img.size,
                created: img.created,
            })
            .collect())
    }

    /// Ensure the ACA base image exists, building it if necessary.
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Path to directory containing Dockerfile (only used if image needs to be built)
    ///
    /// # Errors
    ///
    /// Returns error if image check or build fails.
    pub async fn ensure_aca_base_image(&self, dockerfile_path: Option<&Path>) -> Result<String> {
        if self.image_exists(ACA_BASE_IMAGE).await? {
            info!("ACA base image already exists: {}", ACA_BASE_IMAGE);
            return Ok(ACA_BASE_IMAGE.to_string());
        }

        if let Some(path) = dockerfile_path {
            info!("ACA base image not found, building...");
            self.build_aca_base_image(path, None).await
        } else {
            Err(ContainerError::ConfigError(format!(
                "ACA base image '{}' not found. Please build it first: docker build -t {} -f container/Dockerfile container/",
                ACA_BASE_IMAGE, ACA_BASE_IMAGE
            )))
        }
    }
}

/// Image information.
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// Image ID
    pub id: String,
    /// Repository tags
    pub repo_tags: Vec<String>,
    /// Size in bytes
    pub size: i64,
    /// Creation timestamp
    pub created: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Docker
    async fn test_image_exists() {
        use bollard::Docker;

        let docker = Docker::connect_with_local_defaults().unwrap();
        let builder = ImageBuilder::new(docker);

        // alpine should not exist initially (or might exist)
        let exists = builder.image_exists("alpine:latest").await.unwrap();
        println!("alpine:latest exists: {}", exists);
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_images() {
        use bollard::Docker;

        let docker = Docker::connect_with_local_defaults().unwrap();
        let builder = ImageBuilder::new(docker);

        let images = builder.list_images().await.unwrap();
        println!("Found {} images", images.len());
        for img in images.iter().take(5) {
            println!("  {} - {:?}", img.id, img.repo_tags);
        }
    }
}
