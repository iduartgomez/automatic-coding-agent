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
use {
    aca::cli::{IntelligentParserError, IntelligentTaskParser, TaskAnalysisRequest},
    aca::llm::{
        LLMError, LLMProvider, OpenAIProvider, ProviderConfig, ProviderType, RateLimitConfig,
    },
    serde_json::json,
    std::error::Error,
    std::fs,
    std::path::{Path, PathBuf},
    std::sync::Arc,
};

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
fn should_run_codex_tests() -> bool {
    env_var_truthy("RUN_CODEX_TESTS") || env_var_truthy("CODEX_TEST_REAL")
}

#[cfg(not(target_family = "unix"))]
fn should_run_codex_tests() -> bool {
    false
}

#[cfg(target_family = "unix")]
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let destination = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_all(&path, &destination)?;
        } else {
            fs::copy(&path, &destination)?;
        }
    }

    Ok(())
}

#[cfg(target_family = "unix")]
fn setup_test_workspace(resource_dir: &str) -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_path_buf();
    let resource_path = Path::new("tests/resources").join(resource_dir);

    if resource_path.exists() {
        copy_dir_all(&resource_path, &workspace_path)?;
    }

    Ok((temp_dir, workspace_path))
}

#[cfg(target_family = "unix")]
async fn create_codex_provider(workspace: &Path) -> Result<Arc<dyn LLMProvider>, LLMError> {
    let mut additional_config = HashMap::new();

    if let Ok(cli_path) = std::env::var("CODEX_CLI_PATH") {
        additional_config.insert("cli_path".to_string(), json!(cli_path));
    }
    if let Ok(default_model) = std::env::var("CODEX_DEFAULT_MODEL") {
        additional_config.insert("default_model".to_string(), json!(default_model));
    }
    if let Ok(profile) = std::env::var("CODEX_PROFILE") {
        additional_config.insert("profile".to_string(), json!(profile));
    }

    let provider_config = ProviderConfig {
        provider_type: ProviderType::OpenAICodex,
        api_key: None,
        base_url: None,
        model: None,
        rate_limits: RateLimitConfig::default(),
        additional_config,
    };

    let provider = OpenAIProvider::new(provider_config, workspace.to_path_buf()).await?;
    Ok(Arc::new(provider) as Arc<dyn LLMProvider>)
}

#[cfg(target_family = "unix")]
#[tokio::test]
#[tag(openai_codex)]
#[serial]
async fn test_codex_exec_produces_response() {
    if !should_run_codex_tests() {
        eprintln!("skipping Codex integration test: RUN_CODEX_TESTS not enabled");
        return;
    }

    let workspace = TempDir::new().expect("failed to create temp workspace");
    let working_dir = workspace.path().to_path_buf();

    let cli_path = std::env::var("CODEX_CLI_PATH").unwrap_or_else(|_| "codex".to_string());
    let default_model =
        std::env::var("CODEX_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-5".to_string());

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

    // Create logger for test
    let logger_config = aca::llm::provider_logger::ProviderLoggerConfig {
        enabled: true,
        track_tool_uses: false,
        track_commands: true,
        max_preview_chars: 200,
    };
    let logger = aca::llm::provider_logger::ProviderLogger::new(
        "test-codex",
        logger_config,
        working_dir.join(".aca/logs/codex"),
    )
    .await
    .expect("Failed to create test logger");

    let response = match interface.execute_task_request(request, &logger).await {
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

#[cfg(target_family = "unix")]
#[tokio::test]
#[tag(openai_codex)]
#[serial]
async fn test_codex_intelligent_parser_on_nested_tasks() {
    if !should_run_codex_tests() {
        eprintln!("skipping Codex integration test: RUN_CODEX_TESTS not enabled");
        return;
    }

    let (_temp_dir, workspace_path) = match setup_test_workspace("test6-nested-complex-tasks") {
        Ok(result) => result,
        Err(err) => {
            panic!("Failed to set up test workspace: {err}");
        }
    };

    let provider = match create_codex_provider(&workspace_path).await {
        Ok(provider) => provider,
        Err(LLMError::ProviderUnavailable(msg)) => {
            eprintln!("skipping Codex parser test: provider unavailable ({msg})");
            return;
        }
        Err(LLMError::Authentication(msg)) => {
            eprintln!("skipping Codex parser test: authentication required ({msg})");
            return;
        }
        Err(err) => {
            panic!("Failed to create Codex provider: {err:?}");
        }
    };

    let parser = IntelligentTaskParser::new(provider);

    let task_file = workspace_path.join("main-tasks.md");
    let task_content =
        fs::read_to_string(&task_file).expect("failed to read main-tasks.md for Codex test");

    let request = TaskAnalysisRequest {
        content: task_content,
        source_path: Some(task_file),
        context_hints: vec![
            "enterprise ecommerce platform roadmap".to_string(),
            "focus on nested dependencies and phase planning".to_string(),
        ],
        max_tokens: Some(6000),
    };

    let analysis = match parser.analyze_tasks(request).await {
        Ok(analysis) => analysis,
        Err(IntelligentParserError::LLMError(LLMError::Authentication(msg))) => {
            eprintln!("skipping Codex parser test: authentication required ({msg})");
            return;
        }
        Err(IntelligentParserError::LLMError(LLMError::RateLimit { .. })) => {
            eprintln!("skipping Codex parser test: rate limit hit");
            return;
        }
        Err(IntelligentParserError::LLMError(LLMError::ProviderUnavailable(msg))) => {
            eprintln!("skipping Codex parser test: provider unavailable ({msg})");
            return;
        }
        Err(IntelligentParserError::LLMError(LLMError::Network(msg))) => {
            eprintln!("skipping Codex parser test: network error ({msg})");
            return;
        }
        Err(err) => panic!("Codex intelligent parser failed: {err:?}"),
    };

    assert!(
        !analysis.tasks.is_empty(),
        "Codex analysis should produce at least one task"
    );

    let plan =
        parser.analysis_to_execution_plan(analysis.clone(), Some("Codex Nested Tasks".to_string()));

    assert!(plan.has_tasks(), "Execution plan should contain tasks");
    assert!(
        plan.metadata.name.is_some(),
        "Execution plan should include a name"
    );

    println!(
        "Codex generated {} analyzed tasks for nested complex scenario",
        analysis.tasks.len()
    );
}

#[cfg(not(target_family = "unix"))]
#[tokio::test]
#[tag(openai_codex)]
#[serial]
async fn test_codex_exec_produces_response() {
    eprintln!("skipping Codex integration test: not supported on this platform");
}
