use super::rate_limiter::OpenAIRateLimiter;
use super::types::{OpenAIError, OpenAIRateLimitConfig, OpenAITaskRequest};
use std::collections::HashMap;

fn mock_request(estimated_tokens: u64) -> OpenAITaskRequest {
    OpenAITaskRequest {
        id: uuid::Uuid::new_v4(),
        prompt: "fn main() { println!(\"hello\"); }".to_string(),
        metadata: HashMap::new(),
        model: "o4-mini".to_string(),
        system_message: None,
        estimated_tokens,
    }
}

#[tokio::test]
async fn rate_limiter_grants_permits() {
    let limiter = OpenAIRateLimiter::new(OpenAIRateLimitConfig::default());
    let request = mock_request(500);
    let permit = limiter.acquire_permit(&request).await;
    assert!(permit.is_ok());
    let permit = permit.unwrap();
    assert_eq!(permit.tokens_consumed, 500);
}

#[tokio::test]
async fn rate_limiter_blocks_excess_requests() {
    let mut config = OpenAIRateLimitConfig::default();
    config.max_requests_per_minute = 1;
    config.max_tokens_per_minute = 200;
    config.burst_allowance = 0;

    let limiter = OpenAIRateLimiter::new(config);
    let first = limiter.acquire_permit(&mock_request(100)).await;
    assert!(first.is_ok());

    let second = limiter.acquire_permit(&mock_request(100)).await;
    assert!(matches!(second, Err(OpenAIError::RateLimit { .. })));
}
