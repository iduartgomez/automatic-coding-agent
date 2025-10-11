use aca::openai::{
    OpenAICodexInterface, OpenAIConfig, OpenAIError, OpenAILoggingConfig, OpenAIRateLimitConfig,
    OpenAITaskRequest,
};
use serial_test::serial;
use std::collections::HashMap;
use tempfile::TempDir;
use test_tag::tag;
use uuid::Uuid;

#[cfg(target_family = "unix")]
fn env_var_truthy(key: &str) -> bool {
    matches!(
        std::env::var(key),
        Ok(val)
            if matches!(
                val.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
    )
}

#[cfg(target_family = "unix")]
fn write_stub_cli(path: &std::path::Path) -> std::io::Result<()> {
    use std::fs;
    #[cfg(target_family = "unix")]
    use std::os::unix::fs::PermissionsExt;

    let script = r#"#!/usr/bin/env bash
# Codex CLI stub used for tests. Consumes stdin and emits deterministic JSONL events.
cat >/dev/null
cat <<'JSON'
{"type":"thread.started","thread_id":"stub-thread"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"stub-item","type":"agent_message","text":"Hello from Codex stub!"}}
{"type":"turn.completed","usage":{"input_tokens":12,"cached_input_tokens":0,"output_tokens":6}}
{"type":"run.completed","reason":"completed"}
JSON
"#;

    fs::write(path, script)?;
    #[cfg(target_family = "unix")]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

#[cfg(target_family = "unix")]
#[tokio::test]
#[tag(openai_codex)]
#[serial]
async fn test_codex_exec_produces_response() {
    let workspace = TempDir::new().expect("failed to create temp workspace");
    let working_dir = workspace.path().to_path_buf();
    let use_real_cli = env_var_truthy("CODEX_TEST_REAL") || env_var_truthy("RUN_CODEX_TESTS");

    let (cli_path, default_model, expect_stub_response) = if use_real_cli {
        let cli = std::env::var("CODEX_CLI_PATH").unwrap_or_else(|_| "codex".to_string());
        let model =
            std::env::var("CODEX_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-5-codex".to_string());
        (cli, model, None)
    } else {
        let stub_path = working_dir.join("codex_stub.sh");
        write_stub_cli(&stub_path).expect("failed to write Codex stub CLI");
        (
            stub_path.to_string_lossy().to_string(),
            "codex-stub-model".to_string(),
            Some("Hello from Codex stub!".to_string()),
        )
    };

    let config = OpenAIConfig {
        cli_path,
        default_model: default_model.clone(),
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

    let interface = match OpenAICodexInterface::new(config).await {
        Ok(interface) => interface,
        Err(OpenAIError::CliUnavailable(path)) => {
            eprintln!("skipping Codex test: CLI unavailable at {path}");
            return;
        }
        Err(err) => panic!("failed to initialize Codex interface: {err:?}"),
    };

    let mut metadata = HashMap::new();
    metadata.insert("test_case".to_string(), "codex_exec_smoke".to_string());

    let request = OpenAITaskRequest {
        id: Uuid::new_v4(),
        prompt: "Reply with a short friendly greeting.".to_string(),
        metadata,
        model: default_model,
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

    if let Some(expected) = expect_stub_response {
        assert_eq!(
            response.response_text.trim(),
            expected,
            "stub CLI should return deterministic response"
        );
    }
}

#[cfg(not(target_family = "unix"))]
#[tokio::test]
#[tag(openai_codex)]
#[serial]
async fn test_codex_exec_produces_response() {
    eprintln!("skipping Codex integration test: stub CLI only available on Unix targets");
}
