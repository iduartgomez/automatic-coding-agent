use automatic_coding_agent::integration::AgentConfig;
use tempfile::NamedTempFile;

#[test]
fn test_config_serialization_roundtrip() {
    let original_config = AgentConfig::default();

    // Test serialization to TOML string
    let toml_str = original_config.to_toml_string()
        .expect("Should be able to serialize config to TOML");

    assert!(!toml_str.is_empty(), "TOML string should not be empty");
    assert!(toml_str.contains("workspace_path"), "Should contain workspace_path field");

    // Test deserialization from TOML string
    let deserialized_config = AgentConfig::from_toml_str(&toml_str)
        .expect("Should be able to deserialize TOML string");

    // Verify key fields match
    assert_eq!(original_config.workspace_path, deserialized_config.workspace_path);
    assert_eq!(original_config.setup_commands.len(), deserialized_config.setup_commands.len());
    assert_eq!(
        original_config.session_config.auto_save_interval_minutes,
        deserialized_config.session_config.auto_save_interval_minutes
    );
    assert_eq!(
        original_config.task_config.max_concurrent_tasks,
        deserialized_config.task_config.max_concurrent_tasks
    );
}

#[test]
fn test_config_file_operations() {
    let original_config = AgentConfig::default();

    // Create a temporary file
    let temp_file = NamedTempFile::new()
        .expect("Should be able to create temporary file");
    let temp_path = temp_file.path();

    // Test saving config to file
    original_config.to_toml_file(temp_path)
        .expect("Should be able to save config to file");

    // Test loading config from file
    let loaded_config = AgentConfig::from_toml_file(temp_path)
        .expect("Should be able to load config from file");

    // Verify the loaded config matches the original
    assert_eq!(original_config.workspace_path, loaded_config.workspace_path);
    assert_eq!(original_config.setup_commands.len(), loaded_config.setup_commands.len());
    assert_eq!(
        original_config.session_config.auto_save_interval_minutes,
        loaded_config.session_config.auto_save_interval_minutes
    );
    assert_eq!(
        original_config.task_config.max_concurrent_tasks,
        loaded_config.task_config.max_concurrent_tasks
    );
}

#[test]
fn test_config_toml_structure() {
    let config = AgentConfig::default();
    let toml_str = config.to_toml_string()
        .expect("Should be able to serialize config");

    // Verify TOML contains expected sections
    assert!(toml_str.contains("[session_config]"), "Should contain session_config section");
    assert!(toml_str.contains("[task_config]"), "Should contain task_config section");
    assert!(toml_str.contains("[claude_config"), "Should contain claude_config sections");

    // Verify specific fields are present
    assert!(toml_str.contains("auto_save_interval_minutes"), "Should contain auto_save_interval_minutes");
    assert!(toml_str.contains("max_concurrent_tasks"), "Should contain max_concurrent_tasks");
    assert!(toml_str.contains("workspace_path"), "Should contain workspace_path");
}

#[test]
fn test_config_error_handling() {
    // Test loading from non-existent file
    let result = AgentConfig::from_toml_file("non_existent_file.toml");
    assert!(result.is_err(), "Should fail when loading non-existent file");

    // Test parsing invalid TOML
    let invalid_toml = "invalid toml content [[[";
    let result = AgentConfig::from_toml_str(invalid_toml);
    assert!(result.is_err(), "Should fail when parsing invalid TOML");
}

#[test]
fn test_config_customization() {
    use automatic_coding_agent::session::SessionManagerConfig;
    use automatic_coding_agent::task::TaskManagerConfig;
    use automatic_coding_agent::claude::ClaudeConfig;

    // Create a custom config
    let custom_config = AgentConfig {
        workspace_path: "/custom/workspace".into(),
        session_config: SessionManagerConfig {
            auto_save_interval_minutes: 10,
            auto_checkpoint_interval_minutes: 15,
            ..SessionManagerConfig::default()
        },
        task_config: TaskManagerConfig {
            max_concurrent_tasks: 8,
            auto_retry_failed_tasks: false,
            ..TaskManagerConfig::default()
        },
        claude_config: ClaudeConfig::default(),
        setup_commands: Vec::new(),
    };

    // Test serialization and deserialization of custom config
    let toml_str = custom_config.to_toml_string()
        .expect("Should serialize custom config");

    let deserialized = AgentConfig::from_toml_str(&toml_str)
        .expect("Should deserialize custom config");

    assert_eq!(custom_config.workspace_path, deserialized.workspace_path);
    assert_eq!(
        custom_config.session_config.auto_save_interval_minutes,
        deserialized.session_config.auto_save_interval_minutes
    );
    assert_eq!(
        custom_config.session_config.auto_checkpoint_interval_minutes,
        deserialized.session_config.auto_checkpoint_interval_minutes
    );
    assert_eq!(
        custom_config.task_config.max_concurrent_tasks,
        deserialized.task_config.max_concurrent_tasks
    );
    assert_eq!(
        custom_config.task_config.auto_retry_failed_tasks,
        deserialized.task_config.auto_retry_failed_tasks
    );
}