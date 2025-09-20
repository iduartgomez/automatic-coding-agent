//! Demonstration of error handling strategies for setup commands
//!
//! This example specifically tests different error scenarios and how
//! the system handles them with various strategies.

use automatic_coding_agent::{AgentConfig, AgentSystem};
use automatic_coding_agent::task::{SetupCommand, ErrorHandler, OutputCondition};
use chrono::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to see detailed output
    tracing_subscriber::fmt::init();

    println!("🧪 Error Handling Strategies Demo");
    println!("==================================");

    // Test different error handling scenarios
    let test_scenarios = vec![
        // Scenario 1: Skip strategy
        (
            "Skip Strategy Test",
            vec![SetupCommand::new("failing_command", "false") // `false` always exits with code 1
                .optional()
                .with_error_handler(ErrorHandler::skip("skip_false_command"))]
        ),

        // Scenario 2: Retry strategy
        (
            "Retry Strategy Test",
            vec![SetupCommand::new("retry_test", "sh")
                .with_args(vec!["-c".to_string(), "exit 1".to_string()])
                .optional()
                .with_error_handler(ErrorHandler::retry(
                    "retry_failing_command",
                    2,
                    Duration::milliseconds(100)
                ))]
        ),

        // Scenario 3: Backup strategy with stderr analysis
        (
            "Backup Strategy Test",
            vec![SetupCommand::new("backup_test", "nonexistent-command")
                .optional()
                .with_error_handler(ErrorHandler::backup(
                    "backup_nonexistent",
                    OutputCondition::stderr_contains("command not found"),
                    "echo",
                    vec!["Backup command executed successfully".to_string()]
                ))]
        ),

        // Scenario 4: Backup strategy that should NOT trigger
        (
            "Backup Strategy No-Trigger Test",
            vec![SetupCommand::new("backup_no_trigger", "sh")
                .with_args(vec!["-c".to_string(), "echo 'different error' >&2; exit 1".to_string()])
                .optional()
                .with_error_handler(ErrorHandler::backup(
                    "backup_should_not_trigger",
                    OutputCondition::stderr_contains("specific error text"),
                    "echo",
                    vec!["This backup should NOT run".to_string()]
                ))]
        ),
    ];

    for (scenario_name, setup_commands) in test_scenarios {
        println!("\n📝 Testing: {}", scenario_name);
        println!("{}", "─".repeat(50));

        let config = AgentConfig {
            workspace_path: std::env::temp_dir().join("error_demo"),
            setup_commands,
            ..Default::default()
        };

        let start_time = std::time::Instant::now();

        match AgentSystem::new(config).await {
            Ok(_agent) => {
                let duration = start_time.elapsed();
                println!("✅ Scenario completed successfully in {:?}", duration);
            }
            Err(e) => {
                println!("⚠️  Scenario failed (expected for required commands): {}", e);
            }
        }
    }

    println!("\n🎯 Testing Required Command Failure");
    println!("{}", "─".repeat(50));

    // Test a required command that fails (should cause initialization to fail)
    let failing_config = AgentConfig {
        workspace_path: std::env::temp_dir().join("failing_demo"),
        setup_commands: vec![
            SetupCommand::new("required_failing", "false") // This is required and will fail
        ],
        ..Default::default()
    };

    match AgentSystem::new(failing_config).await {
        Ok(_) => {
            println!("❌ Expected initialization to fail but it succeeded");
        }
        Err(e) => {
            println!("✅ Required command failure correctly prevented initialization: {}", e);
        }
    }

    println!("\n🎉 Error handling demo completed!");
    Ok(())
}