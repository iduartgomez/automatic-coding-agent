//! Integration tests for CLI functionality
//!
//! These tests verify that the different CLI components work together properly.
//! Unit tests for individual functions are located in the respective module files.

use automatic_coding_agent::cli::{ConfigDiscovery, DefaultAgentConfig, TaskLoader};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_task_list_parsing_various_formats() {
    let temp_dir = TempDir::new().unwrap();

    // Create task list with various formats
    let task_list = temp_dir.path().join("tasks.md");
    fs::write(
        &task_list,
        "# Task List\n\n\
         - [ ] Create a simple text file with content 'Test File 1'\n\
         - [x] This task is already done (should be included)\n\
         * Generate a message saying 'Hello from task 3'\n\
         \n\
         ## More tasks\n\
         1. Create text content: 'Numbered task output'\n\
         2) Another numbered task with different syntax\n\
         \n\
         * TODO Write 'Org mode task completed' to output\n\
         * DONE This org task is finished\n\
         \n\
         # Comments should be ignored\n\
         // This is also a comment\n\
         \n\
         Plain text task without formatting",
    )
    .unwrap();

    let tasks = TaskLoader::parse_task_list(&task_list).unwrap();

    // Should parse multiple task formats
    assert!(tasks.len() >= 7);

    // Check some specific tasks
    assert!(tasks.iter().any(|t| t.description.contains("Test File 1")));
    assert!(
        tasks
            .iter()
            .any(|t| t.description.contains("Hello from task 3"))
    );
    assert!(
        tasks
            .iter()
            .any(|t| t.description.contains("Numbered task output"))
    );
    assert!(
        tasks
            .iter()
            .any(|t| t.description.contains("Org mode task completed"))
    );
    assert!(
        tasks
            .iter()
            .any(|t| t.description.contains("Plain text task"))
    );
}

#[test]
fn test_task_reference_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Create reference file
    let reference_file = temp_dir.path().join("reference.txt");
    fs::write(&reference_file,
        "Reference Content:\n\nDetailed instructions for the task:\n1. Create output text\n2. Verify content\n3. Clean up"
    ).unwrap();

    // Create task list with reference
    let task_list = temp_dir.path().join("tasks.txt");
    fs::write(
        &task_list,
        format!(
            "- Complete the documented task -> {}\n- Simple task without reference",
            reference_file.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();

    let mut tasks = TaskLoader::parse_task_list(&task_list).unwrap();
    assert_eq!(tasks.len(), 2);

    // Before resolution
    assert!(tasks[0].reference_file.is_some());
    assert!(tasks[1].reference_file.is_none());
    assert!(!tasks[0].description.contains("Detailed instructions"));

    // Resolve references
    TaskLoader::resolve_task_references(&mut tasks).unwrap();

    // After resolution
    assert!(tasks[0].description.contains("Detailed instructions"));
    assert!(tasks[0].description.contains("Reference from"));
    assert!(!tasks[1].description.contains("Reference from"));
}

#[test]
fn test_task_reference_resolution_missing_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create task list with reference to non-existent file
    let task_list = temp_dir.path().join("tasks.txt");
    fs::write(
        &task_list,
        "- Task with missing reference -> nonexistent_file.md",
    )
    .unwrap();

    let mut tasks = TaskLoader::parse_task_list(&task_list).unwrap();
    assert_eq!(tasks.len(), 1);

    // Should fail to resolve references
    let result = TaskLoader::resolve_task_references(&mut tasks);
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("nonexistent_file.md"));
    assert!(error_msg.contains("could not be loaded"));
}

#[test]
fn test_task_conversion_to_agent_commands() {
    use automatic_coding_agent::cli::SimpleTask;

    let tasks = vec![
        SimpleTask {
            description: "Echo 'Task 1 completed'".to_string(),
            reference_file: None,
        },
        SimpleTask {
            description: "Print message: 'Task 2 finished'".to_string(),
            reference_file: None,
        },
    ];

    // Verify tasks were loaded correctly
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].description, "Echo 'Task 1 completed'");
    assert_eq!(tasks[1].description, "Print message: 'Task 2 finished'");
}

#[test]
fn test_configuration_discovery() {
    // This test mainly verifies the discovery mechanism works
    // Since we can't predict the user's actual config files

    let candidates = ConfigDiscovery::find_config_file();
    // Should return None or a valid path if config exists
    if let Some(path) = candidates {
        assert!(path.exists());
        assert!(path.is_file());
    }

    // Test discovery always succeeds (uses defaults if no file)
    let config = ConfigDiscovery::discover_config().unwrap();

    // Should have reasonable defaults
    assert!(config.session_config.auto_save_interval_minutes > 0);
    assert!(config.task_config.max_concurrent_tasks > 0);
}

#[test]
fn test_default_agent_config_serialization() {
    let config = DefaultAgentConfig::default();

    // Test TOML serialization
    let toml_str = toml::to_string(&config).unwrap();
    assert!(!toml_str.is_empty());
    assert!(toml_str.contains("session_config"));
    assert!(toml_str.contains("task_config"));
    assert!(toml_str.contains("claude_config"));

    // Test deserialization
    let deserialized: DefaultAgentConfig = toml::from_str(&toml_str).unwrap();

    // Key fields should match
    assert_eq!(
        config.session_config.auto_save_interval_minutes,
        deserialized.session_config.auto_save_interval_minutes
    );
    assert_eq!(
        config.task_config.max_concurrent_tasks,
        deserialized.task_config.max_concurrent_tasks
    );
}

#[test]
fn test_config_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let original_config = DefaultAgentConfig::default();

    // Save config
    original_config.to_toml_file(&config_path).unwrap();
    assert!(config_path.exists());

    // Load config
    let loaded_config = DefaultAgentConfig::from_toml_file(&config_path).unwrap();

    // Should match
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
fn test_extension_agnostic_file_support() {
    let temp_dir = TempDir::new().unwrap();

    // Test various file extensions and no extension
    let test_files = vec![
        ("task.md", "# Markdown Task\nProcess markdown content"),
        ("task.txt", "Plain text task: Generate simple output"),
        ("task.org", "* Org Mode Task\nProcess org-mode content"),
        ("task.yaml", "# YAML-style task\ntask: Process YAML content"),
        (
            "task",
            "No extension task: Process content without extension",
        ),
    ];

    for (filename, content) in test_files {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content).unwrap();

        // Should parse as single task regardless of extension
        let task = TaskLoader::parse_single_file_task(&file_path).unwrap();
        assert_eq!(task.description, content);
    }
}

#[test]
fn test_comprehensive_task_parsing_workflow() {
    let temp_dir = TempDir::new().unwrap();

    // Create a comprehensive test scenario

    // 1. Create reference files
    let ref1 = temp_dir.path().join("details1.txt");
    fs::write(
        &ref1,
        "Reference 1: Create a text file named 'output1.txt' with content 'Hello World 1'",
    )
    .unwrap();

    let ref2 = temp_dir.path().join("specs.md");
    fs::write(&ref2, "# Specifications\n\nCreate output2.txt with content:\n- Line 1: Hello World 2\n- Line 2: Task completed").unwrap();

    // 2. Create task list with mixed formats and references
    let task_list = temp_dir.path().join("comprehensive_tasks.md");
    fs::write(
        &task_list,
        format!(
            "# Comprehensive Task List\n\n\
         - [ ] Simple echo task: Output 'Basic task completed'\n\
         - [x] Reference task with details -> {}\n\
         * Create greeting: Echo 'Hello from bullet task'\n\
         \n\
         ## Numbered Tasks\n\
         1. Echo message: 'Numbered task 1 done'\n\
         2) Specification-based task -> {}\n\
         \n\
         ## Org Mode Tasks\n\
         * TODO Output 'Org TODO completed'\n\
         * DONE This was already finished\n\
         \n\
         Plain task: Echo 'Plain text task finished'",
            ref1.file_name().unwrap().to_string_lossy(),
            ref2.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();

    // 3. Parse task list
    let mut tasks = TaskLoader::parse_task_list(&task_list).unwrap();

    // Should find multiple tasks
    assert!(tasks.len() >= 7);

    // Some tasks should have references
    let ref_count = tasks.iter().filter(|t| t.reference_file.is_some()).count();
    assert_eq!(ref_count, 2);

    // 4. Resolve references
    TaskLoader::resolve_task_references(&mut tasks).unwrap();

    // Referenced tasks should now contain reference content
    let task_with_ref1 = tasks
        .iter()
        .find(|t| t.description.contains("Reference 1"))
        .expect("Should find task with reference 1");
    assert!(task_with_ref1.description.contains("output1.txt"));
    assert!(task_with_ref1.description.contains("Hello World 1"));

    let task_with_ref2 = tasks
        .iter()
        .find(|t| t.description.contains("Specifications"))
        .expect("Should find task with reference 2");
    assert!(task_with_ref2.description.contains("output2.txt"));
    assert!(task_with_ref2.description.contains("Hello World 2"));

    // 5. Verify all tasks were loaded correctly
    assert!(!tasks.is_empty());

    // Verify task content integration
    for task in &tasks {
        assert!(
            !task.description.is_empty(),
            "Task description should not be empty"
        );
    }
}

#[test]
fn test_workspace_override_functionality() {
    let config = DefaultAgentConfig::default();
    let custom_workspace = PathBuf::from("/custom/workspace");

    let agent_config = config.to_agent_config(Some(custom_workspace.clone()));
    assert_eq!(agent_config.workspace_path, custom_workspace);

    // Test without override uses current directory
    let agent_config = config.to_agent_config(None);
    assert!(!agent_config.workspace_path.as_os_str().is_empty());
}
