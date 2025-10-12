use crate::llm::types::{LLMError, LLMRequest, LLMResponse, ProviderCapabilities, ProviderStatus};
use futures::future::BoxFuture;
use std::path::PathBuf;
use std::sync::Arc;

/// Generic LLM Provider trait that can be implemented by any LLM service
pub trait LLMProvider: Send + Sync {
    /// Execute a single LLM request
    ///
    /// # Arguments
    /// * `request` - The LLM request to execute
    /// * `session_dir` - Optional session directory for audit logs and coordination with session persistence
    ///
    /// When `session_dir` is provided, providers should write audit logs (stdout, stderr, commands)
    /// to that directory for proper session correlation and audit trail.
    fn execute_request(
        &self,
        request: LLMRequest,
        session_dir: Option<PathBuf>,
    ) -> BoxFuture<'_, Result<LLMResponse, LLMError>>;

    /// Get provider capabilities
    fn get_capabilities(&self) -> BoxFuture<'_, Result<ProviderCapabilities, LLMError>>;

    /// Get current provider health status
    fn get_status(&self) -> BoxFuture<'_, Result<ProviderStatus, LLMError>>;

    /// Test provider connectivity
    fn health_check(&self) -> BoxFuture<'_, Result<(), LLMError>>;

    /// Get provider name/identifier
    fn provider_name(&self) -> &'static str;

    /// Get supported models
    fn list_models(&self) -> BoxFuture<'_, Result<Vec<String>, LLMError>>;

    /// Estimate token count for text (provider-specific tokenization)
    fn estimate_tokens(&self, text: &str) -> u64;

    /// Clean up resources
    fn shutdown(&self) -> BoxFuture<'_, Result<(), LLMError>> {
        Box::pin(async { Ok(()) })
    }
}

/// Factory for creating LLM providers
pub struct LLMProviderFactory;

impl LLMProviderFactory {
    pub async fn create_provider(
        config: crate::llm::types::ProviderConfig,
        workspace_root: PathBuf,
    ) -> Result<Arc<dyn LLMProvider>, LLMError> {
        match config.provider_type {
            crate::llm::types::ProviderType::ClaudeCode => Ok(Arc::new(
                crate::llm::claude_provider::ClaudeProvider::new(config, workspace_root).await?,
            )),
            crate::llm::types::ProviderType::OpenAICodex => Ok(Arc::new(
                crate::llm::openai_provider::OpenAIProvider::new(config, workspace_root).await?,
            )),
            crate::llm::types::ProviderType::Anthropic => {
                // TODO: Implement direct Anthropic API provider
                Err(LLMError::ProviderUnavailable(
                    "Anthropic provider not yet implemented".to_string(),
                ))
            }
            crate::llm::types::ProviderType::LocalModel => {
                // TODO: Implement local model provider (e.g., Ollama)
                Err(LLMError::ProviderUnavailable(
                    "Local model provider not yet implemented".to_string(),
                ))
            }
            crate::llm::types::ProviderType::Custom(name) => Err(LLMError::ProviderUnavailable(
                format!("Custom provider '{}' not implemented", name),
            )),
        }
    }
}
