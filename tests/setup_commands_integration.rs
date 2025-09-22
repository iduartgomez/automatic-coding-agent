use automatic_coding_agent::task::{ErrorHandler, OutputCondition, SetupCommand};
use automatic_coding_agent::{AgentConfig, AgentSystem};
use chrono::Duration;
use std::path::PathBuf;
use test_tag::tag;

#[tokio::test]
#[tag(claude)]
async fn test_simple_successful_command() {
    let setup_commands =
        vec![SetupCommand::new("check_rust", "rustc").with_args(vec!["--version".to_string()])];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("simple_success_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Simple successful command should work");
}

#[tokio::test]
#[tag(claude)]
async fn test_command_with_working_directory() {
    let temp_dir = std::env::temp_dir().join("working_dir_test");
    std::fs::create_dir_all(&temp_dir).expect("Should create test directory");

    let setup_commands = vec![
        SetupCommand::new("list_files", "ls")
            .with_args(vec!["-la".to_string()])
            .with_working_dir(temp_dir.clone()),
    ];

    let config = AgentConfig {
        workspace_path: temp_dir.clone(),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Command with working directory should work");

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
#[tag(claude)]
async fn test_optional_command_with_skip_strategy() {
    let setup_commands = vec![
        SetupCommand::new("optional_check", "which")
            .with_args(vec!["nonexistent-command".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::skip("skip_missing_tool")),
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("optional_skip_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Optional command with skip strategy should allow initialization"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_command_with_retry_strategy() {
    let setup_commands = vec![
        SetupCommand::new("retry_test", "sh")
            .with_args(vec!["-c".to_string(), "exit 1".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::retry(
                "retry_failing",
                2,
                Duration::milliseconds(100),
            )),
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("retry_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Command with retry strategy should eventually succeed or be handled"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_command_with_backup_strategy() {
    let setup_commands = vec![
        SetupCommand::new("check_docker", "docker")
            .with_args(vec!["--version".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "docker_backup",
                OutputCondition::stderr_contains("command not found"),
                "echo",
                vec!["Docker not available - using alternative approach".to_string()],
            )),
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("backup_strategy_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Command with backup strategy should handle missing commands"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_file_creation_command() {
    let temp_file = std::env::temp_dir().join("setup_test_integration.txt");

    // Ensure file doesn't exist before test
    if temp_file.exists() {
        std::fs::remove_file(&temp_file).ok();
    }

    let setup_commands = vec![
        SetupCommand::new("create_temp", "touch")
            .with_args(vec![temp_file.to_string_lossy().to_string()])
            .with_timeout(Duration::seconds(10)),
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("file_creation_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "File creation command should succeed");

    // Verify file was created
    assert!(
        temp_file.exists(),
        "Temporary file should have been created"
    );

    // Cleanup
    std::fs::remove_file(&temp_file).ok();
}

// Note: Timeout tests are commented out due to platform-specific timing behavior
// The timeout functionality is tested in unit tests where we have more control
//
// #[tokio::test]
// async fn test_command_with_timeout() { ... }

#[tokio::test]
#[tag(claude)]
async fn test_multiple_setup_commands() {
    let temp_dir = std::env::temp_dir().join("multiple_commands_test");
    std::fs::create_dir_all(&temp_dir).expect("Should create test directory");

    let setup_commands = vec![
        // Successful command
        SetupCommand::new("echo_test", "echo").with_args(vec!["Hello Setup".to_string()]),
        // Command with working directory
        SetupCommand::new("pwd_test", "pwd").with_working_dir(temp_dir.clone()),
        // Optional command that might fail
        SetupCommand::new("optional_fail", "false")
            .optional()
            .with_error_handler(ErrorHandler::skip("skip_false")),
        // Another successful command
        SetupCommand::new("date_test", "date").with_timeout(Duration::seconds(5)),
    ];

    let config = AgentConfig {
        workspace_path: temp_dir.clone(),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Multiple setup commands should work together"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
#[tag(claude)]
async fn test_command_args_handling() {
    let setup_commands = vec![SetupCommand::new("args_test", "echo").with_args(vec![
        "first".to_string(),
        "second".to_string(),
        "third argument".to_string(),
    ])];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("args_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Command with multiple arguments should work"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_setup_commands_builder_pattern() {
    // Test that the builder pattern works correctly
    let cmd = SetupCommand::new("builder_test", "echo")
        .with_args(vec!["test".to_string()])
        .with_working_dir(PathBuf::from("/tmp"))
        .with_timeout(Duration::seconds(10))
        .optional()
        .with_error_handler(ErrorHandler::skip("test_skip"));

    assert_eq!(cmd.name, "builder_test");
    assert_eq!(cmd.command, "echo");
    assert_eq!(cmd.args, vec!["test".to_string()]);
    assert_eq!(cmd.working_dir, Some(PathBuf::from("/tmp")));
    assert_eq!(cmd.timeout, Some(Duration::seconds(10)));
    assert!(!cmd.required); // optional() sets required to false
    assert!(cmd.error_handler.is_some());

    // Test that this command can be used in a setup
    let setup_commands = vec![cmd];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("builder_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Builder pattern command should work");
}
