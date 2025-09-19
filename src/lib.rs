pub mod claude;
pub mod integration;
pub mod llm;
pub mod session;
pub mod task;

// Re-export main session types
pub use session::{SessionInitOptions, SessionManager, SessionManagerConfig, SessionMetadata};

// Re-export main task types
pub use task::{Task, TaskManager, TaskManagerConfig, TaskPriority, TaskSpec, TaskStatus};

// Re-export main Claude types
pub use claude::{ClaudeCodeInterface, ClaudeConfig};

// Re-export LLM abstraction types
pub use llm::{LLMProvider, LLMRequest, LLMResponse, ProviderConfig, ProviderType};

// Re-export integration types
pub use integration::{AgentConfig, AgentSystem, SystemStatus};
