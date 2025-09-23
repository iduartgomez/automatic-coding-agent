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

    // List checkpoints
    let checkpoints = session_manager.list_checkpoints().await;
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
