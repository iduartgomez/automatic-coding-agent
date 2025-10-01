use super::*;
use crate::env;
use std::path::PathBuf;
use std::time::Duration;
use test_tag::tag;

// NOTE: Tests tagged with #[tag(claude)] require actual Claude Code CLI and API access.
// These tests are automatically excluded from CI via the pattern `--skip "::claude::test"`
// To run Claude integration tests locally: cargo test -- --include claude::test

#[tokio::test]
async fn test_claude_interface_creation() {
    let config = ClaudeConfig::default();
    let interface =
        ClaudeCodeInterface::new(config, PathBuf::from(env::test::DEFAULT_TEST_DIR)).await;
    assert!(interface.is_ok());
}

#[tokio::test]
#[tag(claude)]
async fn test_task_request_execution() {
    let config = ClaudeConfig::default();
    let interface = ClaudeCodeInterface::new(config, PathBuf::from(env::test::DEFAULT_TEST_DIR))
        .await
        .unwrap();

    let request = TaskRequest {
        id: uuid::Uuid::new_v4(),
        task_type: "code_generation".to_string(),
        description: "Create a hello world function".to_string(),
        context: std::collections::HashMap::new(),
        priority: TaskPriority::Medium,
        estimated_tokens: Some(100),
    };

    let response = interface.execute_task_request(request).await;
    assert!(response.is_ok());

    let response = response.unwrap();
    assert!(!response.response_text.is_empty());
    assert!(response.token_usage.total_tokens > 0);
}

#[tokio::test]
#[tag(claude)]
async fn test_rate_limiting() {
    let mut config = ClaudeConfig::default();
    config.rate_limits.max_requests_per_minute = 2;
    config.rate_limits.max_tokens_per_minute = 100;

    let interface = ClaudeCodeInterface::new(config, PathBuf::from(env::test::DEFAULT_TEST_DIR))
        .await
        .unwrap();

    // First request should succeed
    let request1 = TaskRequest {
        id: uuid::Uuid::new_v4(),
        task_type: "test".to_string(),
        description: "Test request 1".to_string(),
        context: std::collections::HashMap::new(),
        priority: TaskPriority::Medium,
        estimated_tokens: Some(50),
    };

    let response1 = interface.execute_task_request(request1).await;
    assert!(response1.is_ok());

    // Second request should succeed
    let request2 = TaskRequest {
        id: uuid::Uuid::new_v4(),
        task_type: "test".to_string(),
        description: "Test request 2".to_string(),
        context: std::collections::HashMap::new(),
        priority: TaskPriority::Medium,
        estimated_tokens: Some(30),
    };

    let response2 = interface.execute_task_request(request2).await;
    assert!(response2.is_ok());

    // Third request should be rate limited
    let request3 = TaskRequest {
        id: uuid::Uuid::new_v4(),
        task_type: "test".to_string(),
        description: "Test request 3".to_string(),
        context: std::collections::HashMap::new(),
        priority: TaskPriority::Medium,
        estimated_tokens: Some(30),
    };

    let response3 = interface.execute_task_request(request3).await;
    assert!(response3.is_err());
    assert!(matches!(
        response3.unwrap_err(),
        ClaudeError::RateLimit { .. }
    ));
}

#[tokio::test]
async fn test_context_management() {
    let config = ClaudeConfig::default();
    let context_manager = ContextManager::new(config.context_config);

    let session_id = uuid::Uuid::new_v4();
    let context = context_manager.get_or_create_context(session_id).await;
    assert_eq!(context.session_id, session_id);
    assert!(context.messages.is_empty());

    let message = ClaudeMessage {
        id: uuid::Uuid::new_v4(),
        role: MessageRole::User,
        content: "Hello, Claude!".to_string(),
        timestamp: chrono::Utc::now(),
        token_count: Some(10),
        metadata: std::collections::HashMap::new(),
    };

    context_manager
        .add_message(session_id, message)
        .await
        .unwrap();

    let updated_context = context_manager.get_context(session_id).await.unwrap();
    assert_eq!(updated_context.messages.len(), 1);
    assert_eq!(updated_context.total_tokens, 10);
}

#[tokio::test]
async fn test_usage_tracking() {
    let config = UsageTrackingConfig {
        track_tokens: true,
        track_costs: true,
        track_performance: true,
        history_retention: Duration::from_secs(86400),
    };

    let usage_tracker = UsageTracker::new(config);
    let session_id = uuid::Uuid::new_v4();

    usage_tracker.start_session(session_id).await;

    let response = TaskResponse {
        task_id: uuid::Uuid::new_v4(),
        response_text: "Test response".to_string(),
        tool_uses: vec![],
        token_usage: TokenUsage {
            input_tokens: 50,
            output_tokens: 30,
            total_tokens: 80,
            estimated_cost: 0.001,
        },
        execution_time: Duration::from_millis(500),
        model_used: "claude-3-mock".to_string(),
    };

    usage_tracker.record_usage(session_id, &response).await;

    let session_usage = usage_tracker.get_session_usage(session_id).await.unwrap();
    assert_eq!(session_usage.token_usage.total_tokens, 80);
    assert_eq!(session_usage.request_count, 1);
    assert_eq!(session_usage.total_cost, 0.001);

    let total_usage = usage_tracker.get_total_usage().await;
    assert_eq!(total_usage.total_tokens, 80);
    assert_eq!(total_usage.total_requests, 1);
}

#[tokio::test]
async fn test_interface_status() {
    let config = ClaudeConfig::default();
    let interface = ClaudeCodeInterface::new(config, PathBuf::from(env::test::DEFAULT_TEST_DIR))
        .await
        .unwrap();

    let status = interface.get_interface_status().await;
    assert!(status.is_healthy);
    assert_eq!(status.session_stats.active_sessions, 0);
    assert_eq!(status.session_stats.idle_sessions, 0);
}

#[tokio::test]
async fn test_simple_task_creation() {
    let config = ClaudeConfig::default();
    let interface = ClaudeCodeInterface::new(config, PathBuf::from(env::test::DEFAULT_TEST_DIR))
        .await
        .unwrap();

    let task_request = interface
        .create_task_from_description("Create a test function", "code_generation")
        .await;

    assert_eq!(task_request.task_type, "code_generation");
    assert_eq!(task_request.description, "Create a test function");
    assert!(task_request.estimated_tokens.is_some());
}
