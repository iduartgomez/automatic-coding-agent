use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

pub type SessionId = Uuid;
pub type MessageId = Uuid;
pub type TaskId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub session_config: SessionConfig,
    pub rate_limits: RateLimitConfig,
    pub context_config: ContextConfig,
    pub usage_tracking: UsageTrackingConfig,
    pub error_config: ErrorRecoveryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub max_concurrent_sessions: u32,
    pub session_timeout: Duration,
    pub context_window_size: u32,
    pub auto_checkpoint_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_tokens_per_minute: u64,
    pub max_requests_per_minute: u64,
    pub burst_allowance: u64,
    pub backoff_multiplier: f64,
    pub max_backoff_delay: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub compression_threshold: f64,
    pub max_history_length: u32,
    pub relevance_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageTrackingConfig {
    pub track_tokens: bool,
    pub track_costs: bool,
    pub track_performance: bool,
    pub history_retention: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    pub max_retries: u32,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_timeout: Duration,
    pub enable_fallback_models: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub id: TaskId,
    pub task_type: String,
    pub description: String,
    pub context: HashMap<String, String>,
    pub priority: TaskPriority,
    pub estimated_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: TaskId,
    pub response_text: String,
    pub tool_uses: Vec<ToolUse>,
    pub token_usage: TokenUsage,
    pub execution_time: Duration,
    pub model_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub success: bool,
    pub execution_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub id: MessageId,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub token_count: Option<u64>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub session_id: SessionId,
    pub messages: Vec<ClaudeMessage>,
    pub total_tokens: u64,
    pub last_activity: DateTime<Utc>,
    pub context_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatePermit {
    pub granted_at: DateTime<Utc>,
    pub tokens_consumed: u64,
    pub permit_id: Uuid,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ClaudeError {
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        reset_time: DateTime<Utc>,
    },
    #[error("Network timeout: {0}")]
    NetworkTimeout(String),
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailure(String),
    #[error("Model overloaded: {0}")]
    ModelOverloaded(String),
    #[error("Context too large: current={current}, max={max}")]
    ContextTooLarge { current: u64, max: u64 },
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: None,
            session_config: SessionConfig {
                max_concurrent_sessions: 3,
                session_timeout: Duration::from_secs(1800), // 30 minutes
                context_window_size: 200000,
                auto_checkpoint_interval: Duration::from_secs(300), // 5 minutes
            },
            rate_limits: RateLimitConfig {
                max_tokens_per_minute: 40000,
                max_requests_per_minute: 50,
                burst_allowance: 5000,
                backoff_multiplier: 2.0,
                max_backoff_delay: Duration::from_secs(600), // 10 minutes
            },
            context_config: ContextConfig {
                compression_threshold: 0.8,
                max_history_length: 100,
                relevance_threshold: 0.3,
            },
            usage_tracking: UsageTrackingConfig {
                track_tokens: true,
                track_costs: true,
                track_performance: true,
                history_retention: Duration::from_secs(86400 * 7), // 7 days
            },
            error_config: ErrorRecoveryConfig {
                max_retries: 3,
                circuit_breaker_threshold: 5,
                circuit_breaker_timeout: Duration::from_secs(300), // 5 minutes
                enable_fallback_models: true,
            },
        }
    }
}
