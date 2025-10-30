//! Integration tests for container orchestration.
//!
//! These tests verify the container module works end-to-end with Docker/Podman.
//! Tests are skipped if Docker/Podman is not available or SKIP_CONTAINER_TESTS=1.

#![cfg(feature = "containers")]

use aca::container::{
    ContainerClient, ContainerConfig, ContainerOrchestrator, ExecConfig, ImageBuilder,
    RuntimeType, ACA_BASE_IMAGE, ACA_BASE_IMAGE_ALPINE,
};
use serial_test::serial;
use std::path::Path;
use test_tag::tag;

/// Check if container tests should run.
fn should_run_container_tests() -> bool {
    // Skip if explicitly disabled
    if let Ok(value) = std::env::var("SKIP_CONTAINER_TESTS") {
        if value == "1" || value.eq_ignore_ascii_case("true") {
            return false;
        }
    }

    // Check if Docker or Podman is available
    std::process::Command::new("docker")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || std::process::Command::new("podman")
            .arg("info")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

/// Cleanup helper - removes container if exists.
async fn cleanup_container(orchestrator: &ContainerOrchestrator, name: &str) {
    let _ = orchestrator.stop_and_remove(name).await;
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_container_client_connection() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests (Docker/Podman not available or SKIP_CONTAINER_TESTS=1)");
        return;
    }

    let client = ContainerClient::new().await;
    assert!(
        client.is_ok(),
        "Failed to connect to Docker/Podman: {:?}",
        client.err()
    );

    let client = client.unwrap();
    let runtime = client.runtime_type().await.expect("Failed to get runtime type");
    assert!(
        matches!(runtime, aca::container::RuntimeType::Docker | aca::container::RuntimeType::Podman),
        "Expected docker or podman, got: {}",
        runtime
    );

    println!("✓ Connected to {} successfully", runtime);
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_image_builder_list_images() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let client = ContainerClient::new().await.expect("Failed to connect");
    let builder = ImageBuilder::new(client.docker().clone());

    let images = builder.list_images().await;
    assert!(images.is_ok(), "Failed to list images: {:?}", images.err());

    let images = images.unwrap();
    println!("✓ Listed {} images", images.len());
    for img in images.iter().take(3) {
        let size_mb = img.size as f64 / 1_048_576.0;
        println!("  - {} ({:.2} MB)", img.repo_tags.join(", "), size_mb);
    }
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_image_builder_check_exists() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let client = ContainerClient::new().await.expect("Failed to connect");
    let builder = ImageBuilder::new(client.docker().clone());

    // Check for a common base image
    let exists = builder.image_exists("alpine:latest").await;
    assert!(
        exists.is_ok(),
        "Failed to check image existence: {:?}",
        exists.err()
    );

    println!("✓ Image existence check works");
}

#[tokio::test]
#[serial]
#[tag(integration, container, slow)]
async fn test_build_aca_base_image() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    // Only run if Dockerfile exists
    let dockerfile_dir = Path::new("container");
    if !dockerfile_dir.exists() || !dockerfile_dir.join("Dockerfile").exists() {
        eprintln!("Skipping base image build (container/Dockerfile not found)");
        return;
    }

    let client = ContainerClient::new().await.expect("Failed to connect");
    let builder = ImageBuilder::new(client.docker().clone());

    println!("Building ACA base image (this may take 10-15 minutes)...");
    let result = builder.build_aca_base_image(dockerfile_dir, None).await;

    assert!(
        result.is_ok(),
        "Failed to build base image: {:?}",
        result.err()
    );

    let image_id = result.unwrap();
    println!("✓ Built ACA base image: {}", image_id);

    // Verify image exists
    let exists = builder.image_exists(ACA_BASE_IMAGE).await.unwrap();
    assert!(exists, "Base image not found after build");
}

#[tokio::test]
#[serial]
#[tag(integration, container, slow)]
async fn test_build_aca_alpine_image() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let dockerfile_dir = Path::new("container");
    if !dockerfile_dir.exists() || !dockerfile_dir.join("Dockerfile.alpine").exists() {
        eprintln!("Skipping Alpine image build (container/Dockerfile.alpine not found)");
        return;
    }

    let client = ContainerClient::new().await.expect("Failed to connect");
    let builder = ImageBuilder::new(client.docker().clone());

    println!("Building ACA Alpine image (this may take 5-7 minutes)...");
    let result = builder
        .build_aca_base_image(dockerfile_dir, Some("alpine"))
        .await;

    assert!(
        result.is_ok(),
        "Failed to build Alpine image: {:?}",
        result.err()
    );

    let image_id = result.unwrap();
    println!("✓ Built ACA Alpine image: {}", image_id);

    // Verify image exists
    let exists = builder.image_exists(ACA_BASE_IMAGE_ALPINE).await.unwrap();
    assert!(exists, "Alpine image not found after build");
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_ensure_aca_base_image() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let dockerfile_dir = Path::new("container");
    if !dockerfile_dir.exists() {
        eprintln!("Skipping auto-build test (container/ directory not found)");
        return;
    }

    let client = ContainerClient::new().await.expect("Failed to connect");
    let builder = ImageBuilder::new(client.docker().clone());

    println!("Ensuring ACA base image (auto-build if needed)...");
    let result = builder.ensure_aca_base_image(Some(dockerfile_dir)).await;

    assert!(
        result.is_ok(),
        "Failed to ensure base image: {:?}",
        result.err()
    );

    let image_tag = result.unwrap();
    println!("✓ Base image ready: {}", image_tag);
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_create_and_start_container() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let orchestrator = ContainerOrchestrator::new().await.expect("Failed to connect");
    let container_name = "aca-test-basic";

    // Cleanup any existing test container
    cleanup_container(&orchestrator, container_name).await;

    // Create minimal config with alpine
    let config = ContainerConfig::builder()
        .image("alpine:latest")
        .build()
        .expect("Failed to build config");

    // Create container
    let create_result = orchestrator
        .create_container(&config, Some(container_name))
        .await;
    assert!(
        create_result.is_ok(),
        "Failed to create container: {:?}",
        create_result.err()
    );

    let container_id = create_result.unwrap();
    println!("✓ Created container: {}", container_id);

    // Start container
    let start_result = orchestrator.start_container(&container_id).await;
    assert!(
        start_result.is_ok(),
        "Failed to start container: {:?}",
        start_result.err()
    );
    println!("✓ Started container");

    // Cleanup
    cleanup_container(&orchestrator, container_name).await;
    println!("✓ Cleaned up container");
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_exec_command_in_container() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let orchestrator = ContainerOrchestrator::new().await.expect("Failed to connect");
    let container_name = "aca-test-exec";

    cleanup_container(&orchestrator, container_name).await;

    // Create and start container
    let config = ContainerConfig::builder()
        .image("alpine:latest")
        
        .build()
        .expect("Failed to build config");

    let container_id = orchestrator
        .create_container(&config, Some(container_name))
        .await
        .expect("Failed to create container");

    orchestrator
        .start_container(&container_id)
        .await
        .expect("Failed to start container");

    println!("✓ Container running, testing exec...");

    // Execute simple command
    let exec_result = orchestrator
        .exec(&container_id, vec!["echo", "Hello from container"])
        .await;

    assert!(
        exec_result.is_ok(),
        "Failed to exec command: {:?}",
        exec_result.err()
    );

    let output = exec_result.unwrap();
    assert_eq!(output.exit_code, Some(0), "Command failed with exit code {:?}", output.exit_code);
    assert!(
        output.stdout.contains("Hello from container"),
        "Expected output not found. Got: {}",
        output.stdout
    );

    println!("✓ Executed command successfully");
    println!("  stdout: {}", output.stdout.trim());

    // Test command that fails
    let fail_result = orchestrator
        .exec(&container_id, vec!["sh", "-c", "exit 42"])
        .await;

    assert!(fail_result.is_ok(), "Exec itself should not error");
    let fail_output = fail_result.unwrap();
    assert_eq!(fail_output.exit_code, Some(42), "Expected exit code 42");
    println!("✓ Exit code handling works correctly");

    cleanup_container(&orchestrator, container_name).await;
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_container_with_bind_mounts() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let orchestrator = ContainerOrchestrator::new().await.expect("Failed to connect");
    let container_name = "aca-test-binds";

    cleanup_container(&orchestrator, container_name).await;

    // Create temp directory for testing
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.txt");
    std::fs::write(&test_file, "test content").expect("Failed to write test file");

    let host_path = temp_dir.path().to_str().unwrap();
    let bind_spec = format!("{}:/workspace:rw", host_path);

    // Create container with bind mount
    let config = ContainerConfig::builder()
        .image("alpine:latest")
        
        .bind(&bind_spec)
        .build()
        .expect("Failed to build config");

    let container_id = orchestrator
        .create_container(&config, Some(container_name))
        .await
        .expect("Failed to create container");

    orchestrator
        .start_container(&container_id)
        .await
        .expect("Failed to start container");

    // Verify file is accessible
    let output = orchestrator
        .exec(&container_id, vec!["cat", "/workspace/test.txt"])
        .await
        .expect("Failed to exec");

    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.stdout.trim(), "test content");
    println!("✓ Bind mount works correctly");

    // Test writing from container
    orchestrator
        .exec(&container_id, vec!["sh", "-c", "echo 'from container' > /workspace/new.txt"])
        .await
        .expect("Failed to write from container");

    let new_file = temp_dir.path().join("new.txt");
    let content = std::fs::read_to_string(&new_file).expect("Failed to read new file");
    assert_eq!(content.trim(), "from container");
    println!("✓ Container can write to bind mount");

    cleanup_container(&orchestrator, container_name).await;
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_container_with_environment_variables() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let orchestrator = ContainerOrchestrator::new().await.expect("Failed to connect");
    let container_name = "aca-test-env";

    cleanup_container(&orchestrator, container_name).await;

    let config = ContainerConfig::builder()
        .image("alpine:latest")
        
        .env("TEST_VAR", "test_value")
        .env("ANOTHER_VAR", "another_value")
        .build()
        .expect("Failed to build config");

    let container_id = orchestrator
        .create_container(&config, Some(container_name))
        .await
        .expect("Failed to create container");

    orchestrator
        .start_container(&container_id)
        .await
        .expect("Failed to start container");

    // Check environment variable
    let output = orchestrator
        .exec(&container_id, vec!["sh", "-c", "echo $TEST_VAR"])
        .await
        .expect("Failed to exec");

    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.stdout.trim(), "test_value");
    println!("✓ Environment variables work correctly");

    cleanup_container(&orchestrator, container_name).await;
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_container_with_resource_limits() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let orchestrator = ContainerOrchestrator::new().await.expect("Failed to connect");
    let container_name = "aca-test-resources";

    cleanup_container(&orchestrator, container_name).await;

    // Create container with resource limits
    let config = ContainerConfig::builder()
        .image("alpine:latest")
        
        .memory_limit(536_870_912) // 512 MB
        .cpu_quota(50_000) // 50% of one CPU
        .build()
        .expect("Failed to build config");

    let container_id = orchestrator
        .create_container(&config, Some(container_name))
        .await
        .expect("Failed to create container");

    orchestrator
        .start_container(&container_id)
        .await
        .expect("Failed to start container");

    println!("✓ Container created with resource limits");

    // Verify container is running
    let output = orchestrator
        .exec(&container_id, vec!["echo", "limits applied"])
        .await
        .expect("Failed to exec");

    assert_eq!(output.exit_code, Some(0));
    println!("✓ Container with resource limits is functional");

    cleanup_container(&orchestrator, container_name).await;
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_exec_config_working_directory() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let orchestrator = ContainerOrchestrator::new().await.expect("Failed to connect");
    let container_name = "aca-test-workdir";

    cleanup_container(&orchestrator, container_name).await;

    let config = ContainerConfig::builder()
        .image("alpine:latest")
        
        .build()
        .expect("Failed to build config");

    let container_id = orchestrator
        .create_container(&config, Some(container_name))
        .await
        .expect("Failed to create container");

    orchestrator
        .start_container(&container_id)
        .await
        .expect("Failed to start container");

    // Create a test directory
    orchestrator
        .exec(&container_id, vec!["mkdir", "-p", "/test/subdir"])
        .await
        .expect("Failed to create dir");

    // Execute with custom working directory
    let exec_config = ExecConfig::builder()
        .cmd(vec!["pwd"])
        .working_dir("/test/subdir")
        .build();

    let output = orchestrator
        .exec_with_config(&container_id, &exec_config)
        .await
        .expect("Failed to exec with config");

    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.stdout.trim(), "/test/subdir");
    println!("✓ Custom working directory works");

    cleanup_container(&orchestrator, container_name).await;
}

#[tokio::test]
#[serial]
#[tag(integration, container)]
async fn test_full_workflow() {
    if !should_run_container_tests() {
        eprintln!("Skipping container tests");
        return;
    }

    let orchestrator = ContainerOrchestrator::new().await.expect("Failed to connect");
    let container_name = "aca-test-workflow";

    cleanup_container(&orchestrator, container_name).await;

    println!("Testing full container workflow...");

    // 1. Create temp workspace
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let workspace = temp_dir.path().to_str().unwrap();

    // 2. Configure container
    let config = ContainerConfig::builder()
        .image("alpine:latest")
        
        .bind(&format!("{}:/workspace:rw", workspace))
        .env("PROJECT_NAME", "test-project")
        .memory_limit(1_073_741_824) // 1 GB
        .build()
        .expect("Failed to build config");

    // 3. Create and start
    let container_id = orchestrator
        .create_container(&config, Some(container_name))
        .await
        .expect("Failed to create container");
    println!("  ✓ Created container");

    orchestrator
        .start_container(&container_id)
        .await
        .expect("Failed to start container");
    println!("  ✓ Started container");

    // 4. Execute multiple commands
    let commands = vec![
        vec!["echo", "Setting up environment..."],
        vec!["mkdir", "-p", "/workspace/src"],
        vec!["sh", "-c", "echo 'fn main() {}' > /workspace/src/main.rs"],
        vec!["cat", "/workspace/src/main.rs"],
    ];

    for cmd in commands {
        let output = orchestrator
            .exec(&container_id, cmd.clone())
            .await
            .expect("Failed to exec");
        assert_eq!(output.exit_code, Some(0), "Command {:?} failed", cmd);
    }
    println!("  ✓ Executed commands successfully");

    // 5. Verify file on host
    let main_rs = temp_dir.path().join("src/main.rs");
    assert!(main_rs.exists(), "File not created on host");
    let content = std::fs::read_to_string(&main_rs).expect("Failed to read");
    assert_eq!(content.trim(), "fn main() {}");
    println!("  ✓ Files visible on host");

    // 6. Cleanup
    orchestrator
        .stop_and_remove(&container_id)
        .await
        .expect("Failed to cleanup");
    println!("  ✓ Cleaned up successfully");

    println!("✓ Full workflow test passed");
}
