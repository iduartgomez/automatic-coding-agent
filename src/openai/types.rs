use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

pub type TaskId = Uuid;

/// Configuration for interacting with the Codex CLI.
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub cli_path: String,
    pub default_model: String,
    pub profile: Option<String>,
    pub working_dir: PathBuf,
    pub extra_args: Vec<String>,
    pub allow_outside_git: bool,
    pub rate_limits: OpenAIRateLimitConfig,
    pub logging: OpenAILoggingConfig,
}

/// Logging configuration for Codex CLI executions.
#[derive(Debug, Clone)]
pub struct OpenAILoggingConfig {
    pub enable_interaction_logs: bool,
    pub max_preview_chars: usize,
}

/// Rate limiting configuration shared with the provider.
#[derive(Debug, Clone)]
pub struct OpenAIRateLimitConfig {
    pub max_tokens_per_minute: u64,
    pub max_requests_per_minute: u64,
    pub burst_allowance: u64,
    pub backoff_multiplier: f64,
    pub max_backoff_delay: Duration,
}

/// Request forwarded to the Codex CLI.
#[derive(Debug, Clone)]
pub struct OpenAITaskRequest {
    pub id: TaskId,
    pub prompt: String,
    pub metadata: HashMap<String, String>,
    pub model: String,
    pub estimated_tokens: u64,
    pub system_message: Option<String>,
}

/// Response returned from the Codex CLI.
#[derive(Debug, Clone)]
pub struct OpenAITaskResponse {
    pub task_id: TaskId,
    pub response_text: String,
    pub token_usage: TokenUsage,
    pub execution_time: Duration,
    pub model_used: String,
    pub finish_reason: Option<String>,
}

/// Token usage reported by Codex.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub cached_prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost: f64,
}

/// Permit returned by the smart rate limiter.
#[derive(Debug, Clone)]
pub struct RatePermit {
    pub granted_at: DateTime<Utc>,
    pub tokens_consumed: u64,
    pub permit_id: Uuid,
}

/// Errors emitted by the Codex CLI integration.
#[derive(Debug, thiserror::Error)]
pub enum OpenAIError {
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        reset_time: Option<DateTime<Utc>>,
    },
    #[error("Codex CLI not found at path: {0}")]
    CliUnavailable(String),
    #[error("Authentication required: {0}")]
    Authentication(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Context too large: current={current}, max={max}")]
    ContextTooLarge { current: u64, max: u64 },
    #[error("Failed to execute Codex CLI: {0}")]
    CliFailed(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Unknown Codex error: {0}")]
    Unknown(String),
}

impl Default for OpenAIRateLimitConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_minute: 60_000,
            max_requests_per_minute: 60,
            burst_allowance: 4_000,
            backoff_multiplier: 2.0,
            max_backoff_delay: Duration::from_secs(300),
        }
    }
}
