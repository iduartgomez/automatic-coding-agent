use automatic_coding_agent::llm::{LLMProvider, LLMRequest, ProviderConfig, ProviderType};
use automatic_coding_agent::llm::claude_provider::ClaudeProvider;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example of using the LLM provider abstraction

    // Create provider configuration
    let config = ProviderConfig {
        provider_type: ProviderType::Claude,
        api_key: Some("mock-api-key".to_string()),
        base_url: None,
        model: Some("claude-3-sonnet".to_string()),
        rate_limits: automatic_coding_agent::llm::types::RateLimitConfig {
            max_requests_per_minute: 50,
            max_tokens_per_minute: 40000,
            burst_allowance: 5000,
        },
        additional_config: HashMap::new(),
    };

    // Create Claude provider
    let provider = ClaudeProvider::new(config).await?;

    println!("Provider: {}", provider.provider_name());

    // Check provider capabilities
    let capabilities = provider.get_capabilities().await?;
    println!("Capabilities: {:#?}", capabilities);

    // Check provider status
    let status = provider.get_status().await?;
    println!("Status: {:#?}", status);

    // Create an LLM request
    let request = LLMRequest {
        id: Uuid::new_v4(),
        prompt: "Write a simple 'Hello, World!' function in Rust".to_string(),
        context: HashMap::new(),
        max_tokens: Some(1000),
        temperature: Some(0.7),
        model_preference: Some("claude-3-sonnet".to_string()),
        system_message: Some("You are a helpful coding assistant.".to_string()),
    };

    // Execute the request
    println!("\nExecuting request...");
    let response = provider.execute_request(request).await?;

    println!("Response from {}: \n{}", response.model_used, response.content);
    println!("Token usage: {:?}", response.token_usage);
    println!("Execution time: {:?}", response.execution_time);

    Ok(())
}

// Example of how you would add other providers:

/*
// OpenAI Provider (future implementation)
let openai_config = ProviderConfig {
    provider_type: ProviderType::OpenAI,
    api_key: Some("sk-...".to_string()),
    base_url: Some("https://api.openai.com/v1".to_string()),
    model: Some("gpt-4".to_string()),
    rate_limits: RateLimitConfig { ... },
    additional_config: HashMap::new(),
};

let openai_provider = OpenAIProvider::new(openai_config).await?;

// Local Model Provider (future implementation)
let local_config = ProviderConfig {
    provider_type: ProviderType::LocalModel,
    api_key: None,
    base_url: Some("http://localhost:11434".to_string()), // Ollama
    model: Some("llama2".to_string()),
    rate_limits: RateLimitConfig { ... },
    additional_config: HashMap::new(),
};

let local_provider = OllamaProvider::new(local_config).await?;

// Usage is identical regardless of provider:
let response = any_provider.execute_request(request).await?;
*/