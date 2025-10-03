//! Claude-specific LLM provider implementation
//!
//! Implements the [`LLMProvider`] trait for Claude, supporting both CLI and API modes.
//!
//! ## Provider Modes
//!
//! ### CLI Mode (Default)
//! - Uses the `claude` CLI command from Claude Code
//! - No API key required
//! - Leverages local Claude Code installation
//! - Best for development and testing
//!
//! ### API Mode
//! - Direct integration with Anthropic API
//! - Requires `ANTHROPIC_API_KEY` environment variable or config
//! - Suitable for production deployments
//! - Enables advanced features and higher rate limits
//!
//! ## Mode Selection
//!
//! Mode is determined by this priority:
//! 1. `additional_config["mode"]` in ProviderConfig ("CLI" or "API")
//! 2. `CLAUDE_MODE` environment variable ("CLI" or "API")
//! 3. Default: CLI mode
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use aca::llm::{ClaudeProvider, LLMProvider, ProviderConfig, ProviderType};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // CLI mode (default, no API key needed)
//!     let config = ProviderConfig {
//!         provider_type: ProviderType::Claude,
//!         ..Default::default()
//!     };
//!     let provider = ClaudeProvider::new(config, PathBuf::from(".")).await?;
//!
//!     // API mode (requires API key)
//!     let mut additional_config = std::collections::HashMap::new();
//!     additional_config.insert("mode".to_string(), serde_json::json!("API"));
//!     let config_api = ProviderConfig {
//!         provider_type: ProviderType::Claude,
//!         api_key: Some("sk-ant-...".to_string()),
//!         additional_config,
//!         ..Default::default()
//!     };
//!     let provider_api = ClaudeProvider::new(config_api, PathBuf::from(".")).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::claude::ClaudeCodeInterface;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{
    ClaudeProviderMode, LLMError, LLMRequest, LLMResponse, ProviderCapabilities, ProviderConfig,
    ProviderStatus, RateLimitStatus,
};
use chrono::Utc;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Claude-specific implementation of [`LLMProvider`]
///
/// Supports both CLI mode (using `claude` command) and API mode (direct Anthropic API).
/// Mode is automatically determined from configuration or environment variables.
pub struct ClaudeProvider {
    claude_interface: ClaudeCodeInterface,
    #[allow(dead_code)]
    config: ProviderConfig,
    mode: ClaudeProviderMode,
}

impl ClaudeProvider {
    pub async fn new(config: ProviderConfig, workspace_root: PathBuf) -> Result<Self, LLMError> {
        // Determine the provider mode from additional_config or environment, default to CLI
        let mode = Self::determine_mode(&config)?;

        // Validate configuration based on mode
        if mode == ClaudeProviderMode::API && config.api_key.is_none() {
            return Err(LLMError::Authentication(
                "API key is required when using Claude in API mode. Either set an API key or use CLI mode (default).".to_string()
            ));
        }

        // Convert ProviderConfig to ClaudeConfig
        let claude_config = crate::claude::ClaudeConfig {
            api_key: config.api_key.clone(),
            endpoint: config.base_url.clone(),
            session_config: crate::claude::SessionConfig {
                max_concurrent_sessions: 3,
                session_timeout: Duration::from_secs(1800),
                context_window_size: 200000,
                auto_checkpoint_interval: Duration::from_secs(300),
            },
            rate_limits: crate::claude::RateLimitConfig {
                max_tokens_per_minute: config.rate_limits.max_tokens_per_minute,
                max_requests_per_minute: config.rate_limits.max_requests_per_minute,
                burst_allowance: config.rate_limits.burst_allowance,
                backoff_multiplier: 2.0,
                max_backoff_delay: Duration::from_secs(600),
            },
            context_config: crate::claude::ContextConfig {
                compression_threshold: 0.8,
                max_history_length: 100,
                relevance_threshold: 0.3,
            },
            usage_tracking: crate::claude::UsageTrackingConfig {
                track_tokens: true,
                track_costs: true,
                track_performance: true,
                history_retention: Duration::from_secs(86400 * 7),
            },
            error_config: crate::claude::ErrorRecoveryConfig {
                max_retries: 3,
                circuit_breaker_threshold: 5,
                circuit_breaker_timeout: Duration::from_secs(300),
                enable_fallback_models: true,
            },
        };

        let claude_interface = ClaudeCodeInterface::new(claude_config, workspace_root)
            .await
            .map_err(|e| {
                LLMError::ProviderSpecific(format!("Failed to initialize Claude: {}", e))
            })?;

        Ok(Self {
            claude_interface,
            config,
            mode,
        })
    }

    /// Get the current provider mode
    pub fn mode(&self) -> &ClaudeProviderMode {
        &self.mode
    }

    /// Determine the Claude provider mode from config or environment
    fn determine_mode(config: &ProviderConfig) -> Result<ClaudeProviderMode, LLMError> {
        // 1. Check additional_config for "mode" key
        if let Some(mode_str) = config
            .additional_config
            .get("mode")
            .and_then(|v| v.as_str())
        {
            return match mode_str.to_lowercase().as_str() {
                "cli" => Ok(ClaudeProviderMode::CLI),
                "api" => Ok(ClaudeProviderMode::API),
                _ => Err(LLMError::InvalidRequest(format!(
                    "Invalid Claude mode '{}'. Must be 'CLI' or 'API'",
                    mode_str
                ))),
            };
        }

        // 2. Check CLAUDE_MODE environment variable
        if let Ok(mode_env) = std::env::var("CLAUDE_MODE") {
            return match mode_env.to_lowercase().as_str() {
                "cli" => Ok(ClaudeProviderMode::CLI),
                "api" => Ok(ClaudeProviderMode::API),
                _ => Err(LLMError::InvalidRequest(format!(
                    "Invalid CLAUDE_MODE environment variable '{}'. Must be 'CLI' or 'API'",
                    mode_env
                ))),
            };
        }

        // 3. Default to CLI mode
        Ok(ClaudeProviderMode::CLI)
    }
}

impl LLMProvider for ClaudeProvider {
    fn execute_request(&self, request: LLMRequest) -> BoxFuture<'_, Result<LLMResponse, LLMError>> {
        Box::pin(async move {
            // Convert LLMRequest to Claude TaskRequest
            let claude_request = crate::claude::TaskRequest {
                id: request.id,
                task_type: "llm_request".to_string(),
                description: request.prompt,
                context: request.context,
                priority: crate::claude::TaskPriority::Medium,
                estimated_tokens: request.max_tokens,
                system_message: request.system_message,
            };

            // Execute via Claude interface
            let claude_response = self
                .claude_interface
                .execute_task_request(claude_request)
                .await
                .map_err(|e| match e {
                    crate::claude::ClaudeError::RateLimit {
                        message,
                        reset_time,
                    } => LLMError::RateLimit {
                        message,
                        reset_time: Some(reset_time),
                    },
                    crate::claude::ClaudeError::AuthenticationFailure(msg) => {
                        LLMError::Authentication(msg)
                    }
                    crate::claude::ClaudeError::InvalidRequest(msg) => {
                        LLMError::InvalidRequest(msg)
                    }
                    crate::claude::ClaudeError::ContextTooLarge { current, max } => {
                        LLMError::ContextTooLarge { current, max }
                    }
                    crate::claude::ClaudeError::NetworkTimeout(msg) => LLMError::Network(msg),
                    _ => LLMError::ProviderSpecific(format!("Claude error: {}", e)),
                })?;

            // Convert Claude response to LLMResponse
            let mut provider_metadata = HashMap::new();
            provider_metadata.insert(
                "model_used".to_string(),
                serde_json::json!(claude_response.model_used),
            );
            provider_metadata.insert(
                "tool_uses".to_string(),
                serde_json::json!(claude_response.tool_uses),
            );

            Ok(LLMResponse {
                request_id: request.id,
                content: claude_response.response_text,
                model_used: claude_response.model_used,
                token_usage: crate::llm::TokenUsage {
                    input_tokens: claude_response.token_usage.input_tokens,
                    output_tokens: claude_response.token_usage.output_tokens,
                    total_tokens: claude_response.token_usage.total_tokens,
                    estimated_cost: claude_response.token_usage.estimated_cost,
                },
                execution_time: claude_response.execution_time,
                provider_metadata,
            })
        })
    }

    fn get_capabilities(&self) -> BoxFuture<'_, Result<ProviderCapabilities, LLMError>> {
        Box::pin(async move {
            Ok(ProviderCapabilities {
                supports_streaming: false, // Mock Claude doesn't support streaming yet
                supports_function_calling: true,
                supports_vision: false, // Not implemented in mock
                max_context_tokens: 200000,
                available_models: vec![
                    "claude-sonnet".to_string(), // Auto-resolves to latest Sonnet
                    "claude-haiku".to_string(),
                    "claude-opus".to_string(),
                ],
            })
        })
    }

    fn get_status(&self) -> BoxFuture<'_, Result<ProviderStatus, LLMError>> {
        Box::pin(async move {
            let claude_status = self.claude_interface.get_interface_status().await;

            Ok(ProviderStatus {
                is_healthy: claude_status.is_healthy,
                last_check: Utc::now(),
                error_count: claude_status.rate_limiter.failure_count,
                average_response_time: Duration::from_millis(500), // Mock average
                rate_limit_status: RateLimitStatus {
                    requests_remaining: claude_status.rate_limiter.available_requests as u64,
                    tokens_remaining: claude_status.rate_limiter.available_tokens,
                    reset_time: claude_status.rate_limiter.last_failure,
                },
            })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, Result<(), LLMError>> {
        Box::pin(async move {
            let status = self.get_status().await?;
            if status.is_healthy {
                Ok(())
            } else {
                Err(LLMError::ProviderUnavailable(
                    "Claude provider is unhealthy".to_string(),
                ))
            }
        })
    }

    fn provider_name(&self) -> &'static str {
        "claude"
    }

    fn list_models(&self) -> BoxFuture<'_, Result<Vec<String>, LLMError>> {
        Box::pin(async move {
            Ok(vec![
                "claude-sonnet".to_string(), // Auto-resolves to latest Sonnet
                "claude-haiku".to_string(),
                "claude-opus".to_string(),
            ])
        })
    }

    fn estimate_tokens(&self, text: &str) -> u64 {
        // Simple estimation: ~4 characters per token
        (text.len() as f64 / 4.0).ceil() as u64
    }

    fn shutdown(&self) -> BoxFuture<'_, Result<(), LLMError>> {
        Box::pin(async move {
            // Claude interface doesn't have explicit shutdown
            Ok(())
        })
    }
}
