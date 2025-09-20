//! Simple test for backup strategy with commands that actually exist

use automatic_coding_agent::{AgentConfig, AgentSystem};
use automatic_coding_agent::task::{SetupCommand, ErrorHandler, OutputCondition};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("🔄 Backup Strategy Test");
    println!("=======================");

    // Test backup strategy with a command that will fail and trigger backup
    let setup_commands = vec![
        SetupCommand::new("failing_with_backup", "sh")
            .with_args(vec!["-c".to_string(), "echo 'command not found' >&2; exit 1".to_string()])
            .optional()
            .with_error_handler(ErrorHandler::backup(
                "backup_on_failure",
                OutputCondition::stderr_contains("command not found"),
                "echo",
                vec!["🎯 Backup command executed successfully!".to_string()]
            ))
    ];

    let config = AgentConfig {
        workspace_path: std::env::temp_dir().join("backup_test"),
        setup_commands,
        ..Default::default()
    };

    match AgentSystem::new(config).await {
        Ok(_) => println!("✅ Backup strategy test completed successfully!"),
        Err(e) => println!("❌ Test failed: {}", e),
    }

    Ok(())
}