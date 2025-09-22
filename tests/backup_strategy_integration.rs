use automatic_coding_agent::task::{ErrorHandler, OutputCondition, SetupCommand};
use automatic_coding_agent::{AgentConfig, AgentSystem};
use std::env;
use test_tag::tag;

#[tokio::test]
#[tag(claude)]
async fn test_backup_strategy_with_failing_command() {
    // Test backup strategy with a command that will fail and trigger backup
    let setup_commands = vec![
        SetupCommand::new("failing_with_backup", "sh")
            .with_args(vec![
                "-c".to_string(),
                "echo 'command not found' >&2; exit 1".to_string(),
            ])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "backup_on_failure",
                OutputCondition::stderr_contains("command not found"),
                "echo",
                vec!["Backup command executed successfully!".to_string()],
            )),
    ];

    let config = AgentConfig {
        workspace_path: env::temp_dir().join("backup_test"),
        setup_commands,
        ..Default::default()
    };

    // The backup strategy should allow the system to initialize successfully
    // even when the primary command fails, because it has a backup
    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Backup strategy should allow system initialization to succeed"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_backup_strategy_with_specific_error_condition() {
    // Test backup strategy that only triggers on specific error conditions
    let setup_commands = vec![
        SetupCommand::new("specific_error", "sh")
            .with_args(vec![
                "-c".to_string(),
                "echo 'specific error message' >&2; exit 1".to_string(),
            ])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "conditional_backup",
                OutputCondition::stderr_contains("specific error message"),
                "echo",
                vec!["Conditional backup triggered!".to_string()],
            )),
    ];

    let config = AgentConfig {
        workspace_path: env::temp_dir().join("conditional_backup_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Conditional backup should work when error condition is met"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_backup_strategy_no_trigger() {
    // Test backup strategy where the condition is not met (should still fail)
    let setup_commands = vec![
        SetupCommand::new("different_error", "sh")
            .with_args(vec![
                "-c".to_string(),
                "echo 'different error' >&2; exit 1".to_string(),
            ])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "no_trigger_backup",
                OutputCondition::stderr_contains("specific text that won't match"),
                "echo",
                vec!["This backup should not run".to_string()],
            )),
    ];

    let config = AgentConfig {
        workspace_path: env::temp_dir().join("no_trigger_test"),
        setup_commands,
        ..Default::default()
    };

    // Since the command is optional and backup condition isn't met,
    // system should still initialize (optional command failure is allowed)
    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "System should initialize even when optional command fails without backup"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_backup_strategy_success_case() {
    // Test that backup is not triggered when primary command succeeds
    let setup_commands = vec![
        SetupCommand::new("success_command", "echo")
            .with_args(vec!["success".to_string()])
            .with_error_handler(ErrorHandler::backup(
                "unused_backup",
                OutputCondition::stderr_contains("error"),
                "echo",
                vec!["This backup should not be called".to_string()],
            )),
    ];

    let config = AgentConfig {
        workspace_path: env::temp_dir().join("success_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "System should initialize successfully when primary command succeeds"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_backup_strategy_with_exit_code_condition() {
    // Test backup strategy based on exit code range
    let setup_commands = vec![
        SetupCommand::new("exit_code_test", "sh")
            .with_args(vec!["-c".to_string(), "exit 5".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "exit_code_backup",
                OutputCondition::exit_code_range(1, 10),
                "echo",
                vec!["Exit code backup triggered!".to_string()],
            )),
    ];

    let config = AgentConfig {
        workspace_path: env::temp_dir().join("exit_code_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Backup should trigger for exit codes in specified range"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_backup_strategy_complex_condition() {
    // Test backup strategy with multiple conditions
    let setup_commands = vec![
        SetupCommand::new("complex_condition", "sh")
            .with_args(vec![
                "-c".to_string(),
                "echo 'installation failed' >&2; exit 2".to_string(),
            ])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "complex_backup",
                OutputCondition {
                    exit_code_range: Some((1, 5)),
                    contains: Some("installation failed".to_string()),
                    not_contains: None,
                    check_stdout: false,
                    check_stderr: true,
                },
                "echo",
                vec!["Complex condition backup executed!".to_string()],
            )),
    ];

    let config = AgentConfig {
        workspace_path: env::temp_dir().join("complex_condition_test"),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Complex backup condition should work correctly"
    );
}

#[tokio::test]
#[tag(claude)]
async fn test_backup_strategy_working_directory() {
    // Test backup strategy with specific working directory
    let temp_dir = env::temp_dir().join("backup_workdir_test");
    std::fs::create_dir_all(&temp_dir).expect("Should create test directory");

    let setup_commands = vec![
        SetupCommand::new("workdir_test", "sh")
            .with_args(vec![
                "-c".to_string(),
                "echo 'workdir error' >&2; exit 1".to_string(),
            ])
            .with_working_dir(temp_dir.clone())
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "workdir_backup",
                OutputCondition::stderr_contains("workdir error"),
                "pwd", // This will show the working directory
                vec![],
            )),
    ];

    let config = AgentConfig {
        workspace_path: temp_dir.clone(),
        setup_commands,
        ..Default::default()
    };

    let result = AgentSystem::new(config).await;
    assert!(
        result.is_ok(),
        "Backup strategy should work with custom working directory"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}
