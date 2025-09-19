use crate::claude::types::{ClaudeError, ErrorRecoveryConfig};
use chrono::{DateTime, Utc};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug)]
pub struct ErrorRecoveryManager {
    config: ErrorRecoveryConfig,
    circuit_breaker: Arc<CircuitBreaker>,
    error_stats: Arc<Mutex<ErrorStatistics>>,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    config: ErrorRecoveryConfig,
    state: Arc<Mutex<CircuitBreakerState>>,
}

#[derive(Debug, Clone)]
pub enum CircuitBreakerState {
    Closed,
    Open { opened_at: DateTime<Utc> },
    #[allow(dead_code)]
    HalfOpen { test_requests: u32 },
}

#[derive(Debug, Default)]
pub struct ErrorStatistics {
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub total_errors: u64,
    pub last_error_time: Option<DateTime<Utc>>,
    pub error_types: std::collections::HashMap<String, u32>,
}

impl ErrorRecoveryManager {
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(config.clone()));
        let error_stats = Arc::new(Mutex::new(ErrorStatistics::default()));

        Self {
            config,
            circuit_breaker,
            error_stats,
        }
    }

    pub async fn execute_with_recovery<F, T, E>(
        &self,
        operation: F,
    ) -> Result<T, ClaudeError>
    where
        F: Fn() -> BoxFuture<'static, Result<T, E>> + Send + Sync,
        T: Send + Sync,
        E: Into<ClaudeError> + Send + Sync,
    {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < self.config.max_retries {
            // Check circuit breaker
            if !self.circuit_breaker.can_proceed().await {
                return Err(ClaudeError::CircuitBreakerOpen);
            }

            match operation().await {
                Ok(result) => {
                    self.record_success().await;
                    self.circuit_breaker.record_success().await;
                    return Ok(result);
                }
                Err(error) => {
                    let claude_error = error.into();
                    attempt += 1;
                    last_error = Some(claude_error.clone());

                    self.record_error(&claude_error).await;
                    self.circuit_breaker.record_failure().await;

                    // Check if we should retry
                    if !self.should_retry(&claude_error, attempt).await {
                        break;
                    }

                    // Apply recovery action if available
                    if let Some(delay) = self.get_recovery_delay(&claude_error, attempt).await {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(ClaudeError::MaxRetriesExceeded))
    }

    async fn should_retry(&self, error: &ClaudeError, attempt: u32) -> bool {
        if attempt >= self.config.max_retries {
            return false;
        }

        match error {
            ClaudeError::RateLimit { .. } => true,
            ClaudeError::NetworkTimeout(_) => true,
            ClaudeError::ServiceUnavailable(_) => true,
            ClaudeError::ModelOverloaded(_) => true,
            ClaudeError::AuthenticationFailure(_) => false,
            ClaudeError::InvalidRequest(_) => false,
            ClaudeError::CircuitBreakerOpen => false,
            ClaudeError::ContextTooLarge { .. } => false, // Needs different handling
            ClaudeError::MaxRetriesExceeded => false,
            ClaudeError::Unknown(_) => attempt < 2, // Only retry once for unknown errors
        }
    }

    async fn get_recovery_delay(&self, error: &ClaudeError, attempt: u32) -> Option<Duration> {
        match error {
            ClaudeError::RateLimit { reset_time, .. } => {
                let now = Utc::now();
                if *reset_time > now {
                    Some(Duration::from_secs(
                        (reset_time.signed_duration_since(now).num_seconds() as u64).min(300)
                    ))
                } else {
                    Some(Duration::from_secs(60)) // Default 1 minute wait
                }
            }
            ClaudeError::ServiceUnavailable(_) | ClaudeError::ModelOverloaded(_) => {
                // Exponential backoff
                let delay = Duration::from_secs(2u64.pow(attempt.min(5)));
                Some(delay)
            }
            ClaudeError::NetworkTimeout(_) => {
                // Linear backoff
                Some(Duration::from_secs(attempt as u64 * 5))
            }
            _ => None,
        }
    }

    async fn record_success(&self) {
        let mut stats = self.error_stats.lock().await;
        stats.consecutive_successes += 1;
        stats.consecutive_failures = 0;
    }

    async fn record_error(&self, error: &ClaudeError) {
        let mut stats = self.error_stats.lock().await;
        stats.consecutive_failures += 1;
        stats.consecutive_successes = 0;
        stats.total_errors += 1;
        stats.last_error_time = Some(Utc::now());

        let error_type = match error {
            ClaudeError::RateLimit { .. } => "RateLimit",
            ClaudeError::NetworkTimeout(_) => "NetworkTimeout",
            ClaudeError::ServiceUnavailable(_) => "ServiceUnavailable",
            ClaudeError::AuthenticationFailure(_) => "AuthenticationFailure",
            ClaudeError::ModelOverloaded(_) => "ModelOverloaded",
            ClaudeError::ContextTooLarge { .. } => "ContextTooLarge",
            ClaudeError::InvalidRequest(_) => "InvalidRequest",
            ClaudeError::CircuitBreakerOpen => "CircuitBreakerOpen",
            ClaudeError::MaxRetriesExceeded => "MaxRetriesExceeded",
            ClaudeError::Unknown(_) => "Unknown",
        };

        *stats.error_types.entry(error_type.to_string()).or_insert(0) += 1;
    }

    pub async fn get_error_statistics(&self) -> ErrorStatistics {
        let stats = self.error_stats.lock().await;
        stats.clone()
    }
}

impl CircuitBreaker {
    fn new(config: ErrorRecoveryConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CircuitBreakerState::Closed)),
        }
    }

    pub async fn can_proceed(&self) -> bool {
        let state = self.state.lock().await;

        match &*state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open { opened_at } => {
                let elapsed = Utc::now().signed_duration_since(*opened_at);
                elapsed >= chrono::Duration::from_std(self.config.circuit_breaker_timeout).unwrap_or_default()
            }
            CircuitBreakerState::HalfOpen { test_requests } => {
                *test_requests < 3 // Allow a few test requests
            }
        }
    }

    pub async fn record_success(&self) {
        let mut state = self.state.lock().await;

        if let CircuitBreakerState::HalfOpen { test_requests } = &*state
            && *test_requests >= 2 {
                // Enough successful test requests, close the circuit
                *state = CircuitBreakerState::Closed;
            }
    }

    pub async fn record_failure(&self) {
        let mut state = self.state.lock().await;

        match &*state {
            CircuitBreakerState::Closed => {
                // For simplicity, open circuit immediately on any failure
                // In production, you'd track failure rate over time
                *state = CircuitBreakerState::Open {
                    opened_at: Utc::now(),
                };
            }
            CircuitBreakerState::HalfOpen { .. } => {
                // Failure during half-open, go back to open
                *state = CircuitBreakerState::Open {
                    opened_at: Utc::now(),
                };
            }
            CircuitBreakerState::Open { .. } => {
                // Already open, update timestamp
                *state = CircuitBreakerState::Open {
                    opened_at: Utc::now(),
                };
            }
        }
    }

    pub async fn force_open(&self) {
        let mut state = self.state.lock().await;
        *state = CircuitBreakerState::Open {
            opened_at: Utc::now(),
        };
    }

    pub async fn force_close(&self) {
        let mut state = self.state.lock().await;
        *state = CircuitBreakerState::Closed;
    }

    pub async fn get_state(&self) -> CircuitBreakerState {
        let state = self.state.lock().await;
        state.clone()
    }
}

impl Clone for ErrorStatistics {
    fn clone(&self) -> Self {
        Self {
            consecutive_failures: self.consecutive_failures,
            consecutive_successes: self.consecutive_successes,
            total_errors: self.total_errors,
            last_error_time: self.last_error_time,
            error_types: self.error_types.clone(),
        }
    }
}