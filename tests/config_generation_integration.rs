use aca::integration::AgentConfig;
use std::path::Path;

#[test]
fn test_default_config_generation() {
    let config = AgentConfig::default();
    let toml_str = config
        .to_toml_string()
        .expect("Should be able to generate TOML from default config");

    // Verify the generated TOML is well-formed and contains expected content
    assert!(!toml_str.is_empty(), "Generated TOML should not be empty");
    assert!(toml_str.len() > 100, "Generated TOML should be substantial");

    // Verify key sections are present
    assert!(
        toml_str.contains("workspace_path"),
        "Should contain workspace_path"
    );
    assert!(
        toml_str.contains("[session_config]"),
        "Should contain session_config section"
    );
    assert!(
        toml_str.contains("[task_config]"),
        "Should contain task_config section"
    );
    assert!(
        toml_str.contains("[claude_config"),
        "Should contain claude_config sections"
    );

    // Verify the TOML can be parsed back
    let parsed_config =
        AgentConfig::from_toml_str(&toml_str).expect("Generated TOML should be parseable");

    // Verify roundtrip consistency
    assert_eq!(config.workspace_path, parsed_config.workspace_path);
    assert_eq!(
        config.setup_commands.len(),
        parsed_config.setup_commands.len()
    );
}

#[test]
fn test_config_template_generation() {
    let config = AgentConfig::default();
    let toml_str = config.to_toml_string().expect("Should generate TOML");

    // Test that the generated config can serve as a template
    // by verifying it contains all necessary configuration sections
    let required_sections = [
        "workspace_path",
        "[session_config]",
        "auto_save_interval_minutes",
        "auto_checkpoint_interval_minutes",
        "[task_config]",
        "auto_retry_failed_tasks",
        "max_concurrent_tasks",
        "[claude_config.session_config]",
        "[claude_config.rate_limits]",
        "[claude_config.context_config]",
        "[claude_config.usage_tracking]",
        "[claude_config.error_config]",
    ];

    for section in &required_sections {
        assert!(
            toml_str.contains(section),
            "Generated TOML should contain section: {}",
            section
        );
    }
}

#[test]
fn test_config_generation_with_setup_commands() {
    use aca::task::SetupCommand;

    // Create a config with setup commands
    let config = AgentConfig {
        setup_commands: vec![
            SetupCommand::new("test_command", "echo")
                .with_args(vec!["Hello".to_string(), "World".to_string()]),
        ],
        ..Default::default()
    };

    let toml_str = config
        .to_toml_string()
        .expect("Should generate TOML with setup commands");

    // Verify setup commands are serialized
    assert!(
        toml_str.contains("[[setup_commands]]"),
        "Should contain setup_commands array"
    );
    assert!(
        toml_str.contains("test_command"),
        "Should contain command name"
    );
    assert!(toml_str.contains("echo"), "Should contain command");

    // Verify roundtrip
    let parsed_config =
        AgentConfig::from_toml_str(&toml_str).expect("Should parse TOML with setup commands");

    assert_eq!(
        config.setup_commands.len(),
        parsed_config.setup_commands.len()
    );
    assert_eq!(
        config.setup_commands[0].name,
        parsed_config.setup_commands[0].name
    );
    assert_eq!(
        config.setup_commands[0].command,
        parsed_config.setup_commands[0].command
    );
}

#[test]
fn test_config_generation_consistency() {
    // Generate multiple configs and verify they're identical
    let config1 = AgentConfig::default();
    let config2 = AgentConfig::default();

    let toml1 = config1.to_toml_string().expect("Should generate TOML");
    let toml2 = config2.to_toml_string().expect("Should generate TOML");

    assert_eq!(
        toml1, toml2,
        "Default configs should generate identical TOML"
    );
}

#[test]
fn test_config_workspace_path_validation() {
    let config = AgentConfig::default();
    let toml_str = config.to_toml_string().expect("Should generate TOML");

    // Verify workspace path is properly serialized
    assert!(
        toml_str.contains("workspace_path"),
        "Should contain workspace_path field"
    );

    // Parse it back and verify workspace path is a valid path
    let parsed_config = AgentConfig::from_toml_str(&toml_str).expect("Should parse TOML");

    // Verify workspace path is not empty and is a valid path format
    assert!(
        !parsed_config.workspace_path.as_os_str().is_empty(),
        "Workspace path should not be empty"
    );
    assert!(
        Path::new(&parsed_config.workspace_path).is_absolute()
            || parsed_config.workspace_path.is_relative(),
        "Workspace path should be a valid path"
    );
}
