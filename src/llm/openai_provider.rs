use crate::llm::provider::LLMProvider;
use crate::llm::types::{
    LLMError, LLMRequest, LLMResponse, ProviderCapabilities, ProviderConfig, ProviderStatus,
    RateLimitStatus,
};
use crate::openai::{
    OpenAICodexInterface, OpenAIConfig, OpenAIError, OpenAILoggingConfig, OpenAIRateLimitConfig,
    OpenAITaskRequest,
};
use chrono::Utc;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// CLI-backed OpenAI Codex provider.
pub struct OpenAIProvider {
    interface: Arc<OpenAICodexInterface>,
    provider_config: ProviderConfig,
    default_model: String,
}

impl OpenAIProvider {
    pub async fn new(config: ProviderConfig, workspace_root: PathBuf) -> Result<Self, LLMError> {
        let openai_config = Self::build_openai_config(&config, &workspace_root)?;
        let default_model = openai_config.default_model.clone();

        let interface = OpenAICodexInterface::new(openai_config)
            .await
            .map_err(Self::map_error)?;

        Ok(Self {
            interface: Arc::new(interface),
            provider_config: config,
            default_model,
        })
    }

    fn build_openai_config(
        config: &ProviderConfig,
        workspace_root: &Path,
    ) -> Result<OpenAIConfig, LLMError> {
        let additional = &config.additional_config;

        let cli_path = additional
            .get("cli_path")
            .and_then(|v| v.as_str())
            .unwrap_or("codex")
            .to_string();

        let default_model = config
            .model
            .clone()
            .or_else(|| {
                additional
                    .get("default_model")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "o4-mini".to_string());

        let profile = additional
            .get("profile")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let allow_outside_git = additional
            .get("allow_outside_git")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let extra_args = additional
            .get("extra_args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|val| val.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let logging = OpenAILoggingConfig {
            enable_interaction_logs: additional
                .get("enable_interaction_logs")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            max_preview_chars: additional
                .get("max_preview_chars")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(600),
        };

        let rate_limits = OpenAIRateLimitConfig {
            max_tokens_per_minute: config.rate_limits.max_tokens_per_minute,
            max_requests_per_minute: config.rate_limits.max_requests_per_minute,
            burst_allowance: config.rate_limits.burst_allowance,
            backoff_multiplier: additional
                .get("backoff_multiplier")
                .and_then(|v| v.as_f64())
                .unwrap_or(2.0),
            max_backoff_delay: Duration::from_secs(
                additional
                    .get("max_backoff_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(300),
            ),
        };

        Ok(OpenAIConfig {
            cli_path,
            default_model,
            profile,
            working_dir: workspace_root.to_path_buf(),
            extra_args,
            allow_outside_git,
            rate_limits,
            logging,
        })
    }

    fn map_error(error: OpenAIError) -> LLMError {
        match error {
            OpenAIError::RateLimit {
                message,
                reset_time,
            } => LLMError::RateLimit {
                message,
                reset_time,
            },
            OpenAIError::Authentication(msg) => LLMError::Authentication(msg),
            OpenAIError::InvalidRequest(msg) => LLMError::InvalidRequest(msg),
            OpenAIError::ContextTooLarge { current, max } => {
                LLMError::ContextTooLarge { current, max }
            }
            OpenAIError::CliUnavailable(msg) => LLMError::ProviderUnavailable(msg),
            OpenAIError::CliFailed(msg) => LLMError::ProviderSpecific(msg),
            OpenAIError::Serialization(msg) => LLMError::ProviderSpecific(msg),
            OpenAIError::Io(err) => LLMError::ProviderSpecific(err.to_string()),
            OpenAIError::Unknown(msg) => LLMError::ProviderSpecific(msg),
        }
    }

    fn build_task_request(
        &self,
        request: LLMRequest,
        session_dir: Option<PathBuf>,
    ) -> (OpenAITaskRequest, Option<PathBuf>) {
        let estimated_tokens = request
            .max_tokens
            .unwrap_or_else(|| self.estimate_tokens(&request.prompt));

        let mut metadata_filtered = HashMap::new();
        for (key, value) in request.context {
            if value.len() > 2048 {
                warn!("Context value for '{}' truncated to 2048 characters", key);
                metadata_filtered.insert(key, value[..2048].to_string());
            } else {
                metadata_filtered.insert(key, value);
            }
        }

        let model = request
            .model_preference
            .clone()
            .or_else(|| self.provider_config.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let task_request = OpenAITaskRequest {
            id: request.id,
            prompt: request.prompt,
            metadata: metadata_filtered,
            model,
            estimated_tokens,
            system_message: request.system_message,
        };

        (task_request, session_dir)
    }
}

impl LLMProvider for OpenAIProvider {
    fn execute_request(
        &self,
        request: LLMRequest,
        session_dir: Option<PathBuf>,
    ) -> BoxFuture<'_, Result<LLMResponse, LLMError>> {
        let interface = Arc::clone(&self.interface);
        let (task_request, session_dir) = self.build_task_request(request, session_dir);

        Box::pin(async move {
            let response = interface
                .execute_task_request(task_request, session_dir.as_deref())
                .await
                .map_err(OpenAIProvider::map_error)?;

            let mut provider_metadata = HashMap::new();
            provider_metadata.insert(
                "finish_reason".to_string(),
                serde_json::json!(response.finish_reason),
            );

            Ok(LLMResponse {
                request_id: response.task_id,
                content: response.response_text,
                model_used: response.model_used,
                token_usage: crate::llm::TokenUsage {
                    input_tokens: response.token_usage.prompt_tokens,
                    output_tokens: response.token_usage.completion_tokens,
                    total_tokens: response.token_usage.total_tokens,
                    estimated_cost: response.token_usage.estimated_cost,
                },
                execution_time: response.execution_time,
                provider_metadata,
            })
        })
    }

    fn get_capabilities(&self) -> BoxFuture<'_, Result<ProviderCapabilities, LLMError>> {
        Box::pin(async move {
            Ok(ProviderCapabilities {
                supports_streaming: false,
                supports_function_calling: false,
                supports_vision: false,
                max_context_tokens: 128_000,
                available_models: vec![
                    "o4-mini".to_string(),
                    "o3-mini".to_string(),
                    "gpt-4.1".to_string(),
                ],
            })
        })
    }

    fn get_status(&self) -> BoxFuture<'_, Result<ProviderStatus, LLMError>> {
        let interface = Arc::clone(&self.interface);
        Box::pin(async move {
            let status = interface.get_interface_status().await;
            Ok(ProviderStatus {
                is_healthy: status.failure_count < 3,
                last_check: Utc::now(),
                error_count: status.failure_count,
                average_response_time: Duration::from_millis(500),
                rate_limit_status: RateLimitStatus {
                    requests_remaining: status.available_requests as u64,
                    tokens_remaining: status.available_tokens,
                    reset_time: status.last_failure,
                },
            })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, Result<(), LLMError>> {
        let interface = Arc::clone(&self.interface);
        Box::pin(async move {
            let status = interface.get_interface_status().await;
            if status.failure_count > 5 {
                Err(LLMError::ProviderUnavailable(
                    "Codex CLI has multiple recent failures".to_string(),
                ))
            } else {
                Ok(())
            }
        })
    }

    fn provider_name(&self) -> &'static str {
        "codex"
    }

    fn list_models(&self) -> BoxFuture<'_, Result<Vec<String>, LLMError>> {
        Box::pin(async move {
            Ok(vec![
                "o4-mini".to_string(),
                "o3-mini".to_string(),
                "gpt-4.1".to_string(),
            ])
        })
    }

    fn estimate_tokens(&self, text: &str) -> u64 {
        (text.len() as f64 / 4.0).ceil() as u64
    }

    fn shutdown(&self) -> BoxFuture<'_, Result<(), LLMError>> {
        Box::pin(async move { Ok(()) })
    }
}
