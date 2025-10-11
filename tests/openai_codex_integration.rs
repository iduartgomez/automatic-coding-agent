use aca::openai::{
    OpenAICodexInterface, OpenAIConfig, OpenAIError, OpenAILoggingConfig, OpenAIRateLimitConfig,
    OpenAITaskRequest,
};
use serial_test::serial;
use std::collections::HashMap;
use tempfile::TempDir;
use test_tag::tag;
use uuid::Uuid;

fn resolve_cli_path() -> String {
    std::env::var("CODEX_CLI_PATH").unwrap_or_else(|_| "codex".to_string())
}

fn resolve_default_model() -> String {
    std::env::var("CODEX_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-5-codex".to_string())
}

#[tokio::test]
#[tag(openai_codex)]
#[serial]
async fn test_codex_exec_produces_response() {
    let workspace = TempDir::new().expect("failed to create temp workspace");
    let working_dir = workspace.path().to_path_buf();

    let config = OpenAIConfig {
        cli_path: resolve_cli_path(),
        default_model: resolve_default_model(),
        profile: std::env::var("CODEX_PROFILE").ok(),
        working_dir: working_dir.clone(),
        extra_args: Vec::new(),
        allow_outside_git: true,
        rate_limits: OpenAIRateLimitConfig::default(),
        logging: OpenAILoggingConfig {
            enable_interaction_logs: false,
            max_preview_chars: 200,
        },
    };

    let interface = OpenAICodexInterface::new(config)
        .await
        .expect("failed to initialize Codex interface");

    let mut metadata = HashMap::new();
    metadata.insert("test_case".to_string(), "codex_exec_smoke".to_string());

    let request = OpenAITaskRequest {
        id: Uuid::new_v4(),
        prompt: "Reply with a short friendly greeting.".to_string(),
        metadata,
        model: resolve_default_model(),
        estimated_tokens: 64,
        system_message: Some("Be concise and informal.".to_string()),
    };

    let response = match interface
        .execute_task_request(request, Some(working_dir.as_path()))
        .await
    {
        Ok(response) => response,
        Err(OpenAIError::Authentication(msg)) => {
            eprintln!("skipping Codex test: authentication required ({msg})");
            return;
        }
        Err(OpenAIError::CliFailed(msg)) if msg.contains("Unsupported model") => {
            eprintln!("skipping Codex test: {msg}");
            return;
        }
        Err(err) => panic!("Codex execution failed: {err:?}"),
    };

    assert!(
        !response.response_text.trim().is_empty(),
        "Codex CLI returned empty response"
    );
    assert!(
        response.token_usage.total_tokens > 0,
        "Codex CLI should report token usage"
    );
}
