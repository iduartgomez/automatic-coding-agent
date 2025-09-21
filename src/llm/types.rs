use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// Generic LLM request that can be implemented by any provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub id: Uuid,
    pub prompt: String,
    pub context: HashMap<String, String>,
    pub max_tokens: Option<u64>,
    pub temperature: Option<f32>,
    pub model_preference: Option<String>,
    pub system_message: Option<String>,
}

/// Generic LLM response from any provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub request_id: Uuid,
    pub content: String,
    pub model_used: String,
    pub token_usage: TokenUsage,
    pub execution_time: Duration,
    pub provider_metadata: HashMap<String, serde_json::Value>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost: f64,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub rate_limits: RateLimitConfig,
    pub additional_config: HashMap<String, serde_json::Value>,
}

/// Supported LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    Claude,
    OpenAI,
    Anthropic,
    LocalModel,
    Custom(String),
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests_per_minute: u64,
    pub max_tokens_per_minute: u64,
    pub burst_allowance: u64,
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub max_context_tokens: u64,
    pub available_models: Vec<String>,
}

/// Provider health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub is_healthy: bool,
    pub last_check: DateTime<Utc>,
    pub error_count: u32,
    pub average_response_time: Duration,
    pub rate_limit_status: RateLimitStatus,
}

/// Rate limiting status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub requests_remaining: u64,
    pub tokens_remaining: u64,
    pub reset_time: Option<DateTime<Utc>>,
}

/// Generic LLM errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum LLMError {
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        reset_time: Option<DateTime<Utc>>,
    },
    #[error("Authentication failed: {0}")]
    Authentication(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Model not available: {0}")]
    ModelUnavailable(String),
    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("Context too large: {current} > {max}")]
    ContextTooLarge { current: u64, max: u64 },
    #[error("Network error: {0}")]
    Network(String),
    #[error("Provider-specific error: {0}")]
    ProviderSpecific(String),
}

impl Default for LLMRequest {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            prompt: String::new(),
            context: HashMap::new(),
            max_tokens: None,
            temperature: None,
            model_preference: None,
            system_message: None,
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::Claude,
            api_key: None,
            base_url: None,
            model: None,
            rate_limits: RateLimitConfig::default(),
            additional_config: HashMap::new(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_minute: 60,
            max_tokens_per_minute: 10000,
            burst_allowance: 10,
        }
    }
}
