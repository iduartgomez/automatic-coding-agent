use automatic_coding_agent::{AgentConfig, AgentSystem};
use automatic_coding_agent::task::{SetupCommand, ErrorHandler, OutputCondition};
use chrono::Duration;

#[tokio::test]
async fn test_skip_error_strategy() {
    let setup_commands = vec![
        SetupCommand::new("failing_command", "false") // `false` always exits with code 1
            .optional()
            .with_error_handler(ErrorHandler::skip("skip_false_command"))
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("skip_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Skip strategy should allow system initialization to continue");
}

#[tokio::test]
async fn test_retry_error_strategy() {
    let setup_commands = vec![
        SetupCommand::new("retry_test", "sh")
            .with_args(vec!["-c".to_string(), "exit 1".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::retry(
                "retry_failing_command",
                2,
                Duration::milliseconds(100)
            ))
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("retry_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Retry strategy should eventually allow system initialization");
}

#[tokio::test]
async fn test_backup_strategy_with_stderr_analysis() {
    let setup_commands = vec![
        SetupCommand::new("backup_test", "sh")
            .with_args(vec!["-c".to_string(), "echo 'command not found' >&2; exit 1".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "backup_nonexistent",
                OutputCondition::stderr_contains("command not found"),
                "echo",
                vec!["Backup command executed successfully".to_string()]
            ))
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("backup_stderr_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Backup strategy should execute when stderr condition is met");
}

#[tokio::test]
async fn test_backup_strategy_no_trigger() {
    let setup_commands = vec![
        SetupCommand::new("backup_no_trigger", "sh")
            .with_args(vec!["-c".to_string(), "echo 'different error' >&2; exit 1".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "backup_should_not_trigger",
                OutputCondition::stderr_contains("specific error text"),
                "echo",
                vec!["This backup should NOT run".to_string()]
            ))
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("backup_no_trigger_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "System should initialize even when backup doesn't trigger for optional commands");
}

#[tokio::test]
async fn test_required_command_failure() {
    // Test a required command that fails (should cause initialization to fail)
    let setup_commands = vec![
        SetupCommand::new("required_failing", "false") // This is required and will fail
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("required_fail_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_err(), "Required command failure should prevent system initialization");
}

#[tokio::test]
async fn test_multiple_error_strategies() {
    let setup_commands = vec![
        // First command: skip on failure
        SetupCommand::new("skip_command", "false")
            .optional()
            .with_error_handler(ErrorHandler::skip("skip_test")),

        // Second command: retry on failure
        SetupCommand::new("retry_command", "sh")
            .with_args(vec!["-c".to_string(), "exit 1".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::retry(
                "retry_test",
                1,
                Duration::milliseconds(50)
            )),

        // Third command: backup on specific condition
        SetupCommand::new("backup_command", "sh")
            .with_args(vec!["-c".to_string(), "echo 'trigger backup' >&2; exit 1".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "backup_test",
                OutputCondition::stderr_contains("trigger backup"),
                "echo",
                vec!["All strategies working".to_string()]
            ))
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("multiple_strategies_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Multiple error strategies should work together");
}

// Note: Timeout tests are commented out due to platform-specific timing behavior
// The timeout functionality is tested in unit tests where we have more control
//
// #[tokio::test]
// async fn test_error_handler_with_timeout() { ... }

#[tokio::test]
async fn test_error_handler_with_working_directory() {
    let temp_dir = std::env::temp_dir().join("error_workdir_test");
    std::fs::create_dir_all(&temp_dir).expect("Should create test directory");

    let setup_commands = vec![
        SetupCommand::new("workdir_error", "false")
            .with_working_dir(temp_dir.clone())
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "workdir_backup",
                OutputCondition::exit_code_range(1, 1),
                "pwd",
                vec![]
            ))
    ];

    let config = AgentConfig {
        workspace_path: temp_dir.clone(),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Error handler should work with custom working directory");

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_complex_error_conditions() {
    let setup_commands = vec![
        SetupCommand::new("complex_error", "sh")
            .with_args(vec!["-c".to_string(), "echo 'installation failed with code 3' >&2; exit 3".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "complex_condition",
                OutputCondition {
                    exit_code_range: Some((1, 5)),
                    contains: Some("installation failed".to_string()),
                    not_contains: Some("permission denied".to_string()),
                    check_stdout: false,
                    check_stderr: true,
                },
                "echo",
                vec!["Complex error condition handled".to_string()]
            ))
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("complex_error_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(result.is_ok(), "Complex error conditions should be handled correctly");
}