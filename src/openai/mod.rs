//! OpenAI Codex integration layer.
//!
//! This module mirrors the structure of the Claude integration while
//! providing CLI-based access to OpenAI's Codex agent.

pub mod codex_interface;
pub mod rate_limiter;
pub mod types;

#[cfg(test)]
mod tests;

pub use codex_interface::OpenAICodexInterface;
pub use rate_limiter::{OpenAIRateLimiter, RateLimiterStatus};
pub use types::*;
