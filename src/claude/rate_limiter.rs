use crate::claude::types::{ClaudeError, RateLimitConfig, RatePermit, TaskRequest};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    state: Arc<Mutex<RateLimiterState>>,
}

#[derive(Debug)]
struct RateLimiterState {
    token_bucket: TokenBucket,
    request_bucket: RequestBucket,
    failure_count: u32,
    last_failure: Option<DateTime<Utc>>,
}

#[derive(Debug)]
struct TokenBucket {
    current_tokens: u64,
    last_refill: DateTime<Utc>,
}

#[derive(Debug)]
struct RequestBucket {
    current_requests: u32,
    last_refill: DateTime<Utc>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let now = Utc::now();
        let state = RateLimiterState {
            token_bucket: TokenBucket {
                current_tokens: config.max_tokens_per_minute,
                last_refill: now,
            },
            request_bucket: RequestBucket {
                current_requests: 0,
                last_refill: now,
            },
            failure_count: 0,
            last_failure: None,
        };

        Self {
            config,
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub async fn acquire_permit(&self, request: &TaskRequest) -> Result<RatePermit, ClaudeError> {
        let estimated_tokens = request.estimated_tokens.unwrap_or(1000);

        // Refill buckets based on time elapsed
        self.refill_buckets().await;

        // Apply adaptive backoff if we've had recent failures
        if let Some(delay) = self.calculate_backoff_delay().await {
            tokio::time::sleep(delay).await;
        }

        // Check if we can proceed
        let mut state = self.state.lock().await;

        // Check request rate limit
        if state.request_bucket.current_requests >= self.config.max_requests_per_minute as u32 {
            let reset_time = state.request_bucket.last_refill + Duration::from_secs(60);
            return Err(ClaudeError::RateLimit {
                message: "Request rate limit exceeded".to_string(),
                reset_time,
            });
        }

        // Check token rate limit
        if state.token_bucket.current_tokens < estimated_tokens {
            let reset_time = state.token_bucket.last_refill + Duration::from_secs(60);
            return Err(ClaudeError::RateLimit {
                message: format!("Token rate limit exceeded. Need {} tokens, have {}",
                               estimated_tokens, state.token_bucket.current_tokens),
                reset_time,
            });
        }

        // Consume tokens and requests
        state.token_bucket.current_tokens -= estimated_tokens;
        state.request_bucket.current_requests += 1;

        Ok(RatePermit {
            granted_at: Utc::now(),
            tokens_consumed: estimated_tokens,
            permit_id: Uuid::new_v4(),
        })
    }

    pub async fn record_success(&self) {
        let mut state = self.state.lock().await;
        state.failure_count = 0;
        state.last_failure = None;
    }

    pub async fn record_failure(&self) {
        let mut state = self.state.lock().await;
        state.failure_count += 1;
        state.last_failure = Some(Utc::now());
    }

    async fn refill_buckets(&self) {
        let mut state = self.state.lock().await;
        let now = Utc::now();

        // Refill token bucket
        let token_elapsed = now.signed_duration_since(state.token_bucket.last_refill);
        if token_elapsed >= chrono::Duration::from_std(Duration::from_secs(60)).unwrap_or_default() {
            state.token_bucket.current_tokens = self.config.max_tokens_per_minute;
            state.token_bucket.last_refill = now;
        }

        // Refill request bucket
        let request_elapsed = now.signed_duration_since(state.request_bucket.last_refill);
        if request_elapsed >= chrono::Duration::from_std(Duration::from_secs(60)).unwrap_or_default() {
            state.request_bucket.current_requests = 0;
            state.request_bucket.last_refill = now;
        }
    }

    async fn calculate_backoff_delay(&self) -> Option<Duration> {
        let state = self.state.lock().await;

        if state.failure_count == 0 {
            return None;
        }

        if let Some(last_failure) = state.last_failure {
            let elapsed = Utc::now().signed_duration_since(last_failure);

            // If it's been a while since the last failure, don't apply backoff
            if elapsed > chrono::Duration::from_std(Duration::from_secs(300)).unwrap_or_default() {
                return None;
            }
        }

        // Calculate exponential backoff with jitter
        let base_delay = Duration::from_secs(1);
        let multiplier = self.config.backoff_multiplier.powi(state.failure_count.min(5) as i32);
        let delay = Duration::from_millis((base_delay.as_millis() as f64 * multiplier) as u64);

        // Add jitter (±10%)
        let jitter = (rand::random::<f64>() - 0.5) * 0.2;
        let jittered_delay = Duration::from_millis(
            ((delay.as_millis() as f64) * (1.0 + jitter)) as u64
        );

        Some(jittered_delay.min(self.config.max_backoff_delay))
    }

    pub async fn get_status(&self) -> RateLimiterStatus {
        let state = self.state.lock().await;
        RateLimiterStatus {
            available_tokens: state.token_bucket.current_tokens,
            available_requests: self.config.max_requests_per_minute as u32 - state.request_bucket.current_requests,
            failure_count: state.failure_count,
            last_failure: state.last_failure,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiterStatus {
    pub available_tokens: u64,
    pub available_requests: u32,
    pub failure_count: u32,
    pub last_failure: Option<DateTime<Utc>>,
}