//! Container image building and management.
//!
//! Provides APIs for building custom Docker images programmatically,
//! including the ACA base image with Claude Code and development tools.

use crate::container::{ContainerError, Result};
use bollard::Docker;
use futures::stream::StreamExt;
use std::path::Path;
use tracing::{debug, info};

/// Default ACA base image name (Ubuntu 22.04 full, ~3-4 GB)
pub const ACA_BASE_IMAGE: &str = "aca-dev:latest";

/// ACA Alpine base image name (Alpine 3.19 lightweight, ~800 MB-1 GB)
pub const ACA_BASE_IMAGE_ALPINE: &str = "aca-dev:alpine";

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
    /// * `tag` - Tag variant for the built image:
    ///   - `None` - Uses `Dockerfile` and tags as "aca-dev:latest"
    ///   - `Some("alpine")` - Uses `Dockerfile.alpine` and tags as "aca-dev:alpine"
    ///   - `Some(custom)` - Uses `Dockerfile` and tags with the custom value
    ///
    /// # Errors
    ///
    /// Returns error if Docker CLI is not available or build fails.
    pub async fn build_aca_base_image(
        &self,
        dockerfile_path: &Path,
        tag: Option<&str>,
    ) -> Result<String> {
        // Handle Alpine variant: when tag is "alpine", use Dockerfile.alpine and proper image tag
        let (image_tag, dockerfile_name) = match tag {
            Some("alpine") => (ACA_BASE_IMAGE_ALPINE, "Dockerfile.alpine"),
            Some(custom_tag) => (custom_tag, "Dockerfile"),
            None => (ACA_BASE_IMAGE, "Dockerfile"),
        };
        info!("Building ACA base image: {} using Docker CLI", image_tag);

        // Use Docker CLI for building
        use tokio::process::Command;

        let output = Command::new("docker")
            .arg("build")
            .arg("-t")
            .arg(image_tag)
            .arg("-f")
            .arg(dockerfile_path.join(dockerfile_name))
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
                status_code: 404, ..
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
    /// This function will automatically build the image if it doesn't exist,
    /// using the provided Dockerfile path or the default `container/` directory.
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Optional path to directory containing Dockerfile.
    ///   If None, defaults to "container/" in current directory.
    ///
    /// # Errors
    ///
    /// Returns error if image check or build fails.
    pub async fn ensure_aca_base_image(&self, dockerfile_path: Option<&Path>) -> Result<String> {
        if self.image_exists(ACA_BASE_IMAGE).await? {
            info!("ACA base image already exists: {}", ACA_BASE_IMAGE);
            return Ok(ACA_BASE_IMAGE.to_string());
        }

        // Determine dockerfile path
        let path = dockerfile_path.unwrap_or_else(|| Path::new("container"));

        if !path.exists() {
            return Err(ContainerError::ConfigError(format!(
                "Dockerfile directory not found at '{}'. \n\
                 To build the ACA base image, ensure the container/ directory exists with:\n\
                 - container/Dockerfile\n\
                 - container/entrypoint.sh\n\
                 Or manually build: docker build -t {} -f container/Dockerfile container/",
                path.display(),
                ACA_BASE_IMAGE
            )));
        }

        info!(
            "ACA base image '{}' not found. Building automatically from {}...",
            ACA_BASE_IMAGE,
            path.display()
        );

        self.build_aca_base_image(path, None).await
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
