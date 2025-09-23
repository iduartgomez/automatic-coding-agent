//! # Provider-Agnostic LLM Interface
//!
//! Abstraction layer supporting multiple LLM providers (Claude, OpenAI, local models)
//! with unified API, automatic fallback, and provider-specific optimizations.
//!
//! ## Core Components
//!
//! - **[`LLMProvider`]**: Universal trait for all LLM provider implementations
//! - **`ClaudeProvider`**: Claude-specific implementation with Claude Code integration
//! - **`LLMProviderFactory`**: Factory for creating provider instances
//! - **Provider Types**: Request/response types and configuration structures
//!
//! ## Key Features
//!
//! ### 🔌 Multi-Provider Support
//! - **Claude**: Full integration with Claude Code interface
//! - **OpenAI**: GPT-3.5, GPT-4, and other OpenAI models (planned)
//! - **Anthropic API**: Direct Anthropic API integration (planned)
//! - **Local Models**: Ollama, LocalAI, and other local inference (planned)
//! - **Custom Providers**: Extensible architecture for custom implementations
//!
//! ### 🔀 Unified Interface
//! - Consistent API across all providers
//! - Standardized request/response format
//! - Provider-agnostic error handling
//! - Common configuration patterns
//!
//! ### 🛡️ Reliability Features
//! - Automatic fallback between providers
//! - Provider health monitoring and status checking
//! - Circuit breaker patterns for failed providers
//! - Graceful degradation on provider unavailability
//!
//! ### ⚡ Performance Optimization
//! - Provider-specific rate limiting and cost optimization
//! - Capability detection (streaming, function calling, vision)
//! - Model selection based on task requirements
//! - Token estimation and cost prediction
//!
//! ### 🔧 Configuration Management
//! - Flexible provider configuration system
//! - Environment-based configuration loading
//! - Runtime provider switching and hot-swapping
//! - Provider-specific optimization settings
//!
//! ## Provider Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐
//! │   Application   │───▶│   LLMProvider    │ (Trait)
//! │     Logic       │    │    Interface     │
//! └─────────────────┘    └──────────────────┘
//!                                 │
//!                    ┌────────────┼────────────┐
//!                    │            │            │
//!            ┌───────▼──────┐ ┌───▼───┐ ┌─────▼─────┐
//!            │ClaudeProvider│ │OpenAI │ │LocalModel │
//!            │              │ │Provider│ │ Provider  │
//!            └──────────────┘ └───────┘ └───────────┘
//! ```
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use aca::llm::{
//!     LLMProvider, LLMRequest, ProviderConfig, ProviderType, ClaudeProvider
//! };
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Configure Claude provider
//!     let config = ProviderConfig {
//!         provider_type: ProviderType::Claude,
//!         api_key: Some("your-api-key".to_string()),
//!         model: Some("claude-3-sonnet".to_string()),
//!         ..Default::default()
//!     };
//!
//!     // Create provider instance
//!     let provider = ClaudeProvider::new(config).await?;
//!
//!     // Create a request
//!     let request = LLMRequest {
//!         id: uuid::Uuid::new_v4(),
//!         prompt: "Write a Hello World function in Rust".to_string(),
//!         max_tokens: Some(1000),
//!         temperature: Some(0.7),
//!         context: HashMap::new(),
//!         ..Default::default()
//!     };
//!
//!     // Execute request (works with any provider)
//!     let response = provider.execute_request(request).await?;
//!     println!("Response: {}", response.content);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Adding New Providers
//!
//! To add a new provider, implement the [`LLMProvider`] trait:
//!
//! ```rust,ignore
//! use futures::future::BoxFuture;
//! use crate::llm::{LLMProvider, LLMRequest, LLMResponse, LLMError};
//!
//! pub struct CustomProvider {
//!     config: ProviderConfig,
//!     // ... provider-specific fields
//! }
//!
//! impl LLMProvider for CustomProvider {
//!     fn execute_request(&self, request: LLMRequest) -> BoxFuture<'_, Result<LLMResponse, LLMError>> {
//!         Box::pin(async move {
//!             // Implement provider-specific logic
//!             todo!("Implement request execution")
//!         })
//!     }
//!
//!     // ... implement other required methods
//! }
//! ```

/// Claude-specific LLM provider implementation.
///
/// Integrates with the Claude Code interface to provide full Claude
/// functionality including context management, rate limiting, and error recovery.
pub mod claude_provider;

/// Core LLM provider trait and factory.
///
/// Defines the universal [`LLMProvider`] trait that all provider implementations
/// must satisfy, plus the factory for creating provider instances.
pub mod provider;

/// Provider-agnostic types and configuration.
///
/// Common data types, request/response structures, error types,
/// and configuration options used across all LLM providers.
pub mod types;

pub use claude_provider::ClaudeProvider;
pub use provider::LLMProvider;
pub use types::*;
