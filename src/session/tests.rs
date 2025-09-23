use crate::env;
use crate::session::*;
use crate::task::manager::TaskManagerConfig;
use chrono::{Duration, Utc};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;

/// Helper function to create a test session directory
fn create_test_session_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Helper function to create a test session state
fn create_test_session_state() -> SessionState {
    let mut metadata = SessionMetadata::new("Test Session".to_string(), PathBuf::from(env::test::DEFAULT_TEST_DIR));
    metadata.total_tasks = 5;
    metadata.completed_tasks = 3;
    metadata.failed_tasks = 1;

    let task_tree = crate::task::tree::TaskTree::new();

    SessionState {
        metadata,
        task_tree,
        execution_context: ExecutionContext::default(),
        file_system_state: FileSystemState::default(),
    }
}

#[tokio::test]
async fn test_session_metadata_creation() {
    let workspace_root = PathBuf::from("/test/workspace");
    let metadata = SessionMetadata::new("Test Session".to_string(), workspace_root.clone());

    assert_eq!(metadata.name, "Test Session");
    assert_eq!(metadata.workspace_root, workspace_root);
    assert_eq!(metadata.total_tasks, 0);
    assert_eq!(metadata.completed_tasks, 0);
    assert_eq!(metadata.failed_tasks, 0);
    assert!(metadata.is_compatible());
}

#[tokio::test]
async fn test_session_metadata_checkpoint_management() {
    let mut metadata = SessionMetadata::new("Test Session".to_string(), PathBuf::from(env::test::DEFAULT_TEST_DIR));

    let checkpoint_info = CheckpointInfo {
        id: "test_checkpoint".to_string(),
        created_at: Utc::now(),
        description: "Test checkpoint".to_string(),
        task_count: 10,
        size_bytes: 1024,
        is_automatic: false,
        trigger_reason: CheckpointTrigger::Manual {
            reason: "Testing".to_string(),
        },
    };

    metadata.add_checkpoint(checkpoint_info.clone());

    assert_eq!(metadata.checkpoints.len(), 1);
    assert_eq!(metadata.latest_checkpoint().unwrap().id, "test_checkpoint");
}

#[tokio::test]
async fn test_persistence_manager_creation() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = PersistenceConfig::default();
    let persistence = PersistenceManager::new(session_dir, "test-session", config);

    assert!(persistence.is_ok());
}

#[tokio::test]
async fn test_session_state_save_and_load() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = PersistenceConfig::default();
    let persistence = PersistenceManager::new(session_dir, "test-session", config).unwrap();

    let test_state = create_test_session_state();

    // Save session state
    let save_result = persistence.save_session_state(&test_state).await;
    assert!(save_result.is_ok());

    let save_result = save_result.unwrap();
    assert!(save_result.success);
    assert!(save_result.bytes_written > 0);

    // Load session state
    let loaded_state = persistence.load_session_state().await;
    assert!(loaded_state.is_ok());

    let loaded_state = loaded_state.unwrap();
    assert_eq!(loaded_state.metadata.name, test_state.metadata.name);
    assert_eq!(
        loaded_state.metadata.total_tasks,
        test_state.metadata.total_tasks
    );
    assert_eq!(
        loaded_state.metadata.completed_tasks,
        test_state.metadata.completed_tasks
    );
}

#[tokio::test]
async fn test_checkpoint_creation_and_restoration() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = PersistenceConfig::default();
    let persistence = PersistenceManager::new(session_dir, "test-session", config).unwrap();

    let test_state = create_test_session_state();

    // Create checkpoint
    let checkpoint_result = persistence
        .create_checkpoint(
            &test_state,
            "Test checkpoint".to_string(),
            CheckpointTrigger::Manual {
                reason: "Testing".to_string(),
            },
        )
        .await;

    assert!(checkpoint_result.is_ok());
    let checkpoint_info = checkpoint_result.unwrap();
    assert_eq!(checkpoint_info.description, "Test checkpoint");
    assert!(!checkpoint_info.is_automatic);

    // Restore from checkpoint
    let restored_state = persistence
        .restore_from_checkpoint(&checkpoint_info.id)
        .await;

    assert!(restored_state.is_ok());
    let restored_state = restored_state.unwrap();
    assert_eq!(restored_state.metadata.name, test_state.metadata.name);
}

#[tokio::test]
async fn test_checkpoint_listing() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = PersistenceConfig::default();
    let persistence = PersistenceManager::new(session_dir, "test-session", config).unwrap();

    let test_state = create_test_session_state();

    // Create multiple checkpoints
    for i in 0..3 {
        let description = format!("Test checkpoint {}", i);
        let _ = persistence
            .create_checkpoint(
                &test_state,
                description,
                CheckpointTrigger::Automatic {
                    trigger: AutoTrigger::TaskCompletion { count: i as u32 },
                },
            )
            .await
            .unwrap();
    }

    // List checkpoints
    let checkpoints = persistence.list_checkpoints().await;
    assert!(checkpoints.is_ok());

    let checkpoints = checkpoints.unwrap();
    assert_eq!(checkpoints.len(), 3);
}

#[tokio::test]
async fn test_recovery_manager_creation() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let persistence_config = PersistenceConfig::default();
    let persistence =
        PersistenceManager::new(session_dir, "test-session", persistence_config).unwrap();

    let recovery_config = RecoveryConfig::default();
    let recovery = RecoveryManager::new(persistence, recovery_config);

    assert!(recovery.should_auto_recover());
}

#[tokio::test]
async fn test_session_state_validation() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let persistence_config = PersistenceConfig::default();
    let persistence =
        PersistenceManager::new(session_dir, "test-session", persistence_config).unwrap();

    let recovery_config = RecoveryConfig::default();
    let recovery = RecoveryManager::new(persistence, recovery_config);

    let test_state = create_test_session_state();

    // Validate session state
    let validation_result = recovery.validate_session_state(&test_state).await;
    assert!(validation_result.is_ok());

    let validation_result = validation_result.unwrap();
    assert!(validation_result.is_valid);
    assert!(validation_result.errors.is_empty());
}

#[tokio::test]
async fn test_recovery_from_checkpoint() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let persistence_config = PersistenceConfig::default();
    let persistence = PersistenceManager::new(
        session_dir.clone(),
        "test-session",
        persistence_config.clone(),
    )
    .unwrap();

    let recovery_config = RecoveryConfig::default();
    let recovery = RecoveryManager::new(
        PersistenceManager::new(session_dir, "test-session", persistence_config).unwrap(),
        recovery_config,
    );

    let test_state = create_test_session_state();

    // Create checkpoint
    let checkpoint_info = persistence
        .create_checkpoint(
            &test_state,
            "Recovery test checkpoint".to_string(),
            CheckpointTrigger::Manual {
                reason: "Testing recovery".to_string(),
            },
        )
        .await
        .unwrap();

    // Test recovery
    let recovery_result = recovery.recover_from_checkpoint(&checkpoint_info.id).await;

    assert!(recovery_result.is_ok());
    let recovery_result = recovery_result.unwrap();
    assert!(recovery_result.success);
    assert!(recovery_result.recovered_state.is_some());
}

#[tokio::test]
async fn test_session_manager_creation() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Test Session Manager".to_string(),
        description: Some("Test session for manager".to_string()),
        workspace_root: temp_dir.path().to_path_buf(),
        task_manager_config: TaskManagerConfig::default(),
        persistence_config: PersistenceConfig::default(),
        recovery_config: RecoveryConfig::default(),
        enable_auto_save: false, // Disable for test
        restore_from_checkpoint: None,
    };

    let session_manager = SessionManager::new(session_dir, config, init_options).await;
    assert!(session_manager.is_ok());

    let session_manager = session_manager.unwrap();
    let status = session_manager.get_status().await;
    assert!(status.is_ok());

    let status = status.unwrap();
    assert_eq!(status.name, "Test Session Manager");
    assert!(!status.is_auto_save_active);
}

#[tokio::test]
async fn test_session_manager_checkpoint_operations() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Checkpoint Test Session".to_string(),
        workspace_root: temp_dir.path().to_path_buf(),
        enable_auto_save: false,
        ..Default::default()
    };

    let session_manager = SessionManager::new(session_dir, config, init_options)
        .await
        .unwrap();

    // Create a manual checkpoint
    let checkpoint_result = session_manager
        .create_checkpoint("Manual test checkpoint".to_string())
        .await;

    assert!(checkpoint_result.is_ok());
    let checkpoint_info = checkpoint_result.unwrap();
    assert_eq!(checkpoint_info.description, "Manual test checkpoint");

    // List checkpoints (session-specific for this test)
    let checkpoints = session_manager.list_checkpoints(false).await;
    assert!(checkpoints.is_ok());

    let checkpoints = checkpoints.unwrap();
    assert_eq!(checkpoints.len(), 1);
    assert_eq!(checkpoints[0].id, checkpoint_info.id);
}

#[tokio::test]
async fn test_session_manager_save_and_validate() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Save Test Session".to_string(),
        workspace_root: temp_dir.path().to_path_buf(),
        enable_auto_save: false,
        ..Default::default()
    };

    let session_manager = SessionManager::new(session_dir, config, init_options)
        .await
        .unwrap();

    // Save session
    let save_result = session_manager.save_session().await;
    assert!(save_result.is_ok());

    let save_result = save_result.unwrap();
    assert!(save_result.success);

    // Validate session
    let validation_result = session_manager.validate_session().await;
    assert!(validation_result.is_ok());

    let validation_result = validation_result.unwrap();
    assert!(validation_result.is_valid);
}

#[tokio::test]
async fn test_session_manager_auto_save_control() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Auto Save Test Session".to_string(),
        workspace_root: temp_dir.path().to_path_buf(),
        enable_auto_save: true,
        ..Default::default()
    };

    let session_manager = SessionManager::new(session_dir, config, init_options)
        .await
        .unwrap();

    // Check initial auto-save status
    let status = session_manager.get_status().await.unwrap();
    assert!(status.is_auto_save_active);

    // Disable auto-save
    let disable_result = session_manager.set_auto_save_enabled(false).await;
    assert!(disable_result.is_ok());

    // Check disabled status
    let status = session_manager.get_status().await.unwrap();
    assert!(!status.is_auto_save_active);

    // Re-enable auto-save
    let enable_result = session_manager.set_auto_save_enabled(true).await;
    assert!(enable_result.is_ok());

    let status = session_manager.get_status().await.unwrap();
    assert!(status.is_auto_save_active);
}

#[tokio::test]
async fn test_session_statistics() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Statistics Test Session".to_string(),
        workspace_root: temp_dir.path().to_path_buf(),
        enable_auto_save: false,
        ..Default::default()
    };

    let session_manager = SessionManager::new(session_dir, config, init_options)
        .await
        .unwrap();

    // Get session statistics
    let statistics = session_manager.get_session_statistics().await;
    assert!(statistics.is_ok());

    let statistics = statistics.unwrap();
    assert_eq!(statistics.total_checkpoints, 0);
    assert!(statistics.total_execution_time >= Duration::zero());
}

#[tokio::test]
async fn test_session_manager_graceful_shutdown() {
    let temp_dir = create_test_session_dir();
    let session_dir = temp_dir.path().to_path_buf();

    let config = SessionManagerConfig::default();
    let init_options = SessionInitOptions {
        name: "Shutdown Test Session".to_string(),
        workspace_root: temp_dir.path().to_path_buf(),
        enable_auto_save: true,
        ..Default::default()
    };

    let session_manager = SessionManager::new(session_dir, config, init_options)
        .await
        .unwrap();

    // Test graceful shutdown
    let shutdown_result = session_manager.shutdown().await;
    assert!(shutdown_result.is_ok());

    // Verify auto-save is disabled after shutdown
    let status = session_manager.get_status().await.unwrap();
    assert!(!status.is_auto_save_active);
}

#[tokio::test]
async fn test_list_all_checkpoints_in_workspace() {
    let temp_dir = create_test_session_dir();
    let workspace_root = temp_dir.path().to_path_buf();

    // Create a few sessions with checkpoints
    let sessions_dir = env::sessions_dir_path(&workspace_root);
    std::fs::create_dir_all(&sessions_dir).unwrap();

    // Create session 1 with 2 checkpoints
    let session1_dir = sessions_dir.join("session-1");
    let session1_checkpoints_dir = session1_dir.join("checkpoints");
    std::fs::create_dir_all(&session1_checkpoints_dir).unwrap();

    let checkpoint1_data = serde_json::json!({
        "metadata": {
            "checkpoints": [{
                "id": "checkpoint_001",
                "created_at": "2025-09-23T10:00:00Z",
                "description": "First checkpoint",
                "task_count": 1,
                "size_bytes": 1000,
                "is_automatic": false,
                "trigger_reason": {"Manual": {"reason": "Test checkpoint"}}
            }]
        }
    });
    std::fs::write(
        session1_checkpoints_dir.join("checkpoint_001.json"),
        serde_json::to_string_pretty(&checkpoint1_data).unwrap()
    ).unwrap();

    let checkpoint2_data = serde_json::json!({
        "metadata": {
            "checkpoints": [{
                "id": "checkpoint_002",
                "created_at": "2025-09-23T11:00:00Z",
                "description": "Second checkpoint",
                "task_count": 2,
                "size_bytes": 2000,
                "is_automatic": true,
                "trigger_reason": {"Automatic": {"trigger": {"TimeInterval": {"minutes": 30}}}}
            }]
        }
    });
    std::fs::write(
        session1_checkpoints_dir.join("checkpoint_002.json"),
        serde_json::to_string_pretty(&checkpoint2_data).unwrap()
    ).unwrap();

    // Create session 2 with 1 checkpoint
    let session2_dir = sessions_dir.join("session-2");
    let session2_checkpoints_dir = session2_dir.join("checkpoints");
    std::fs::create_dir_all(&session2_checkpoints_dir).unwrap();

    let checkpoint3_data = serde_json::json!({
        "metadata": {
            "checkpoints": [{
                "id": "checkpoint_003",
                "created_at": "2025-09-23T12:00:00Z",
                "description": "Third checkpoint",
                "task_count": 0,
                "size_bytes": 500,
                "is_automatic": false,
                "trigger_reason": {"Manual": {"reason": "Another test checkpoint"}}
            }]
        }
    });
    std::fs::write(
        session2_checkpoints_dir.join("checkpoint_003.json"),
        serde_json::to_string_pretty(&checkpoint3_data).unwrap()
    ).unwrap();

    // Test listing all checkpoints across sessions using a temporary session manager
    let temp_session_config = SessionManagerConfig::default();
    let temp_init_options = SessionInitOptions {
        name: "Test Session".to_string(),
        workspace_root: workspace_root.clone(),
        ..Default::default()
    };
    let temp_session = SessionManager::new(workspace_root.clone(), temp_session_config, temp_init_options).await.unwrap();
    let checkpoints = temp_session.list_checkpoints(true).await.unwrap();

    // Should return all checkpoints sorted by creation time (most recent first)
    assert_eq!(checkpoints.len(), 3);
    assert_eq!(checkpoints[0].id, "checkpoint_003"); // Most recent (12:00)
    assert_eq!(checkpoints[1].id, "checkpoint_002"); // Middle (11:00)
    assert_eq!(checkpoints[2].id, "checkpoint_001"); // Oldest (10:00)

    // Verify checkpoint details
    assert_eq!(checkpoints[0].description, "Third checkpoint");
    assert_eq!(checkpoints[1].description, "Second checkpoint");
    assert_eq!(checkpoints[2].description, "First checkpoint");

    // Test with empty workspace
    let empty_temp_dir = create_test_session_dir();
    let empty_workspace = empty_temp_dir.path().to_path_buf();
    let empty_session_config = SessionManagerConfig::default();
    let empty_init_options = SessionInitOptions {
        name: "Empty Test Session".to_string(),
        workspace_root: empty_workspace.clone(),
        ..Default::default()
    };
    let empty_session = SessionManager::new(empty_workspace.clone(), empty_session_config, empty_init_options).await.unwrap();
    let empty_checkpoints = empty_session.list_checkpoints(true).await.unwrap();
    assert_eq!(empty_checkpoints.len(), 0);
}

#[tokio::test]
async fn test_create_checkpoint_in_latest_session() {
    let temp_dir = create_test_session_dir();
    let workspace_root = temp_dir.path().to_path_buf();

    // Create sessions directory structure
    let sessions_dir = env::sessions_dir_path(&workspace_root);
    std::fs::create_dir_all(&sessions_dir).unwrap();

    // Create an older session
    let old_session_dir = sessions_dir.join("old-session");
    std::fs::create_dir_all(&old_session_dir).unwrap();

    // Wait a moment to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Create a newer session
    let new_session_dir = sessions_dir.join("new-session");
    std::fs::create_dir_all(&new_session_dir).unwrap();

    // Test creating a checkpoint in the latest session using a session manager
    let checkpoint_session_config = SessionManagerConfig::default();
    let checkpoint_init_options = SessionInitOptions {
        name: "Checkpoint Test Session".to_string(),
        workspace_root: workspace_root.clone(),
        ..Default::default()
    };
    let checkpoint_session = SessionManager::new(workspace_root.clone(), checkpoint_session_config, checkpoint_init_options).await.unwrap();
    let checkpoint = checkpoint_session.create_checkpoint_in_latest_session_of_workspace(
        "Test manual checkpoint".to_string()
    ).await.unwrap();

    // Verify checkpoint was created
    assert!(checkpoint.id.starts_with("checkpoint_"));
    assert_eq!(checkpoint.description, "Test manual checkpoint");
    assert!(!checkpoint.is_automatic);

    // Verify checkpoint file was created somewhere in the workspace
    // Let's find where it was actually created
    let sessions_dir = env::sessions_dir_path(&workspace_root);
    let mut found_checkpoint_file = false;
    if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
        for entry in entries.flatten() {
            let session_path = entry.path();
            if session_path.is_dir() {
                let checkpoints_dir = session_path.join("checkpoints");
                if checkpoints_dir.exists() {
                    let checkpoint_file = checkpoints_dir.join(format!("{}.json", checkpoint.id));
                    if checkpoint_file.exists() {
                        found_checkpoint_file = true;
                        break;
                    }
                }
            }
        }
    }
    assert!(found_checkpoint_file,
           "Checkpoint file {} should exist in some session directory", checkpoint.id);

    // Verify checkpoint can be listed
    let all_checkpoints = checkpoint_session.list_checkpoints(true).await.unwrap();
    assert_eq!(all_checkpoints.len(), 1);
    assert_eq!(all_checkpoints[0].id, checkpoint.id);
    assert_eq!(all_checkpoints[0].description, "Test manual checkpoint");

    // Test behavior when starting with a fresh session
    // Since creating a SessionManager automatically creates a session,
    // the checkpoint creation should succeed even in a "new" workspace
    let fresh_temp_dir = create_test_session_dir();
    let fresh_workspace = fresh_temp_dir.path().to_path_buf();
    let fresh_session_config = SessionManagerConfig::default();
    let fresh_init_options = SessionInitOptions {
        name: "Fresh Test Session".to_string(),
        workspace_root: fresh_workspace.clone(),
        ..Default::default()
    };
    let fresh_session = SessionManager::new(fresh_workspace.clone(), fresh_session_config, fresh_init_options).await.unwrap();
    let result = fresh_session.create_checkpoint_in_latest_session_of_workspace(
        "Should succeed in fresh session".to_string()
    ).await;
    assert!(result.is_ok(), "Creating checkpoint in fresh session should succeed");
}

#[tokio::test]
async fn test_manual_checkpoint_visibility_integration() {
    // This test simulates the original Issue #13: Manual checkpoint visibility
    let temp_dir = create_test_session_dir();
    let workspace_root = temp_dir.path().to_path_buf();

    // First, simulate creating auto-checkpoints during normal session operation
    let sessions_dir = env::sessions_dir_path(&workspace_root);
    let session_dir = sessions_dir.join("test-session");
    let checkpoints_dir = session_dir.join("checkpoints");
    std::fs::create_dir_all(&checkpoints_dir).unwrap();

    // Create an auto-checkpoint (simulating what happens during task execution)
    let auto_checkpoint_data = serde_json::json!({
        "metadata": {
            "checkpoints": [{
                "id": "checkpoint_auto_001",
                "created_at": "2025-09-23T10:00:00Z",
                "description": "auto_save",
                "task_count": 1,
                "size_bytes": 1500,
                "is_automatic": true,
                "trigger_reason": {"Automatic": {"trigger": {"TimeInterval": {"minutes": 5}}}}
            }]
        }
    });
    std::fs::write(
        checkpoints_dir.join("checkpoint_auto_001.json"),
        serde_json::to_string_pretty(&auto_checkpoint_data).unwrap()
    ).unwrap();

    // Create a session manager for testing
    let test_session_config = SessionManagerConfig::default();
    let test_init_options = SessionInitOptions {
        name: "Integration Test Session".to_string(),
        workspace_root: workspace_root.clone(),
        ..Default::default()
    };
    let test_session = SessionManager::new(workspace_root.clone(), test_session_config, test_init_options).await.unwrap();

    // Test 1: List checkpoints should show the auto-checkpoint
    let checkpoints_before = test_session.list_checkpoints(true).await.unwrap();
    assert_eq!(checkpoints_before.len(), 1);
    assert_eq!(checkpoints_before[0].id, "checkpoint_auto_001");
    assert_eq!(checkpoints_before[0].description, "auto_save");

    // Test 2: Create a manual checkpoint (this was failing before the fix)
    let manual_checkpoint = test_session.create_checkpoint_in_latest_session_of_workspace(
        "Manual test checkpoint".to_string()
    ).await.unwrap();

    // Test 3: List checkpoints should now show BOTH auto and manual checkpoints
    let checkpoints_after = test_session.list_checkpoints(true).await.unwrap();
    assert_eq!(checkpoints_after.len(), 2);

    // Should be sorted by creation time (manual checkpoint is newer)
    assert_eq!(checkpoints_after[0].id, manual_checkpoint.id);
    assert_eq!(checkpoints_after[0].description, "Manual test checkpoint");
    assert!(!checkpoints_after[0].is_automatic);

    assert_eq!(checkpoints_after[1].id, "checkpoint_auto_001");
    assert_eq!(checkpoints_after[1].description, "auto_save");
    assert!(checkpoints_after[1].is_automatic);

    // Test 4: Create another manual checkpoint to verify continued functionality
    let second_manual = test_session.create_checkpoint_in_latest_session_of_workspace(
        "Second manual checkpoint".to_string()
    ).await.unwrap();

    let final_checkpoints = test_session.list_checkpoints(true).await.unwrap();
    assert_eq!(final_checkpoints.len(), 3);
    assert_eq!(final_checkpoints[0].id, second_manual.id); // Most recent
    assert_eq!(final_checkpoints[1].id, manual_checkpoint.id); // Middle
    assert_eq!(final_checkpoints[2].id, "checkpoint_auto_001"); // Oldest
}
