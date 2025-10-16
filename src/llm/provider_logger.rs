//! Standardized logging abstraction for LLM provider interactions.
//!
//! This module provides a unified logging interface that all LLM providers use
//! to ensure consistent audit trails, reproducibility, and debugging capabilities
//! across different provider implementations (Claude, OpenAI, etc.).
//!
//! ## Features
//!
//! - **Standardized file naming**: `{provider}-{timestamp}-{request-id}.*`
//! - **Complete audit trail**: 5 files per execution
//!   - `.log`: Human-readable summary with timestamps
//!   - `.stdout.json`: Full JSON/JSONL output (no truncation)
//!   - `.stderr.txt`: Error output
//!   - `.command.sh`: Reproducible command for debugging
//!   - `.tools.json`: Tool uses (when enabled)
//! - **Consistent metadata**: Token usage, costs, execution time, model info
//! - **Configurable tracking**: Enable/disable specific logging features
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use aca::llm::provider_logger::{ProviderLogger, ProviderLoggerConfig, LogContext};
//! use uuid::Uuid;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ProviderLoggerConfig {
//!     enabled: true,
//!     track_tool_uses: true,
//!     track_commands: true,
//!     max_preview_chars: 500,
//! };
//!
//! let logger = ProviderLogger::new(
//!     "claude",
//!     config,
//!     PathBuf::from("/workspace/.aca/sessions/session-123/logs")
//! ).await?;
//!
//! let request_id = Uuid::new_v4();
//! let ctx = LogContext::new(request_id, "gpt-4");
//!
//! // Log command execution
//! logger.log_command_start(&ctx, "claude --print --model sonnet").await?;
//!
//! // Log events during execution
//! logger.log_event(&ctx, "Processing request").await?;
//!
//! // Save full outputs
//! logger.save_stdout(&ctx, b"{\"result\": \"Hello\"}").await?;
//! logger.save_stderr(&ctx, b"").await?;
//!
//! // Save reproducible command
//! logger.save_command_script(&ctx, "#!/bin/bash\nclaude --print -- \"prompt\"").await?;
//!
//! // Log completion with metadata
//! logger.log_completion(&ctx, 100, 50, 150, 0.002, 5.5).await?;
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

/// Configuration for provider logging behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLoggerConfig {
    /// Enable/disable all logging
    pub enabled: bool,
    /// Track tool uses (Write, Edit, Bash, etc.)
    pub track_tool_uses: bool,
    /// Save reproducible command scripts
    pub track_commands: bool,
    /// Maximum characters to show in log previews
    pub max_preview_chars: usize,
}

impl Default for ProviderLoggerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            track_tool_uses: true,
            track_commands: true,
            max_preview_chars: 500,
        }
    }
}

/// Context for a single request being logged.
#[derive(Debug, Clone)]
pub struct LogContext {
    /// Unique request identifier
    pub request_id: Uuid,
    /// Timestamp when request started
    pub started_at: DateTime<Utc>,
    /// Model being used
    pub model: String,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl LogContext {
    pub fn new(request_id: Uuid, model: impl Into<String>) -> Self {
        Self {
            request_id,
            started_at: Utc::now(),
            model: model.into(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Tool use information captured during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Tool name (e.g., "Write", "Edit", "Bash")
    pub tool_name: String,
    /// Tool input parameters
    pub input: serde_json::Value,
    /// Tool output (if available)
    pub output: Option<serde_json::Value>,
    /// Timestamp when tool was invoked
    pub timestamp: DateTime<Utc>,
}

/// Standardized logger for LLM provider interactions.
///
/// Provides dependency injection point for uniform logging across all providers.
#[derive(Debug, Clone)]
pub struct ProviderLogger {
    /// Provider name (e.g., "claude", "openai", "codex")
    provider_name: String,
    /// Logger configuration
    config: ProviderLoggerConfig,
    /// Base directory for logs
    logs_dir: PathBuf,
}

impl ProviderLogger {
    /// Create a new provider logger.
    ///
    /// # Arguments
    /// * `provider_name` - Name of the provider (e.g., "claude", "openai")
    /// * `config` - Logging configuration
    /// * `logs_dir` - Base directory for logs (usually `.aca/sessions/{id}/logs/{provider}_interactions`)
    pub async fn new(
        provider_name: impl Into<String>,
        config: ProviderLoggerConfig,
        logs_dir: PathBuf,
    ) -> Result<Self, std::io::Error> {
        let provider_name = provider_name.into();

        // Create logs directory if it doesn't exist
        if config.enabled {
            fs::create_dir_all(&logs_dir).await?;
        }

        Ok(Self {
            provider_name,
            config,
            logs_dir,
        })
    }

    /// Generate base path for all files related to a request.
    fn base_path(&self, ctx: &LogContext) -> PathBuf {
        let timestamp = ctx.started_at.format("%Y%m%dT%H%M%S%.3f");
        self.logs_dir.join(format!(
            "{}-{}-{}",
            self.provider_name, timestamp, ctx.request_id
        ))
    }

    /// Get path to the main log file.
    fn log_file_path(&self, ctx: &LogContext) -> PathBuf {
        self.base_path(ctx).with_extension("log")
    }

    /// Get path to the stdout file.
    fn stdout_file_path(&self, ctx: &LogContext) -> PathBuf {
        self.base_path(ctx).with_extension("stdout.json")
    }

    /// Get path to the stderr file.
    fn stderr_file_path(&self, ctx: &LogContext) -> PathBuf {
        self.base_path(ctx).with_extension("stderr.txt")
    }

    /// Get path to the command script file.
    fn command_file_path(&self, ctx: &LogContext) -> PathBuf {
        self.base_path(ctx).with_extension("command.sh")
    }

    /// Get path to the tools file.
    fn tools_file_path(&self, ctx: &LogContext) -> PathBuf {
        self.base_path(ctx).with_extension("tools.json")
    }

    /// Log command execution start.
    pub async fn log_command_start(
        &self,
        ctx: &LogContext,
        command_summary: &str,
    ) -> Result<(), std::io::Error> {
        if !self.config.enabled {
            return Ok(());
        }

        let log_file = self.log_file_path(ctx);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file)
            .await?;

        let header = format!(
            "{} Provider Interaction Log\n\
             Provider: {}\n\
             Request ID: {}\n\
             Model: {}\n\
             Started: {}\n\
             {}\n\
             [{}] Executing command: {}\n",
            self.provider_name.to_uppercase(),
            self.provider_name,
            ctx.request_id,
            ctx.model,
            ctx.started_at.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
            "=".repeat(80),
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            command_summary
        );

        file.write_all(header.as_bytes()).await?;
        Ok(())
    }

    /// Log a general event.
    pub async fn log_event(&self, ctx: &LogContext, message: &str) -> Result<(), std::io::Error> {
        if !self.config.enabled {
            return Ok(());
        }

        let log_file = self.log_file_path(ctx);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .await?;

        let log_line = format!(
            "[{}] {}\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            message
        );

        file.write_all(log_line.as_bytes()).await?;
        Ok(())
    }

    /// Save full stdout output.
    pub async fn save_stdout(&self, ctx: &LogContext, stdout: &[u8]) -> Result<(), std::io::Error> {
        if !self.config.enabled || stdout.is_empty() {
            return Ok(());
        }

        let stdout_file = self.stdout_file_path(ctx);
        fs::write(&stdout_file, stdout).await?;

        // Log preview in main log file
        let preview = self.create_preview(stdout);
        self.log_event(ctx, &format!("STDOUT ({}B): {}", stdout.len(), preview))
            .await?;

        Ok(())
    }

    /// Save full stderr output.
    pub async fn save_stderr(&self, ctx: &LogContext, stderr: &[u8]) -> Result<(), std::io::Error> {
        if !self.config.enabled || stderr.is_empty() {
            return Ok(());
        }

        let stderr_file = self.stderr_file_path(ctx);
        fs::write(&stderr_file, stderr).await?;

        // Log preview in main log file
        let preview = self.create_preview(stderr);
        self.log_event(ctx, &format!("STDERR ({}B): {}", stderr.len(), preview))
            .await?;

        Ok(())
    }

    /// Save reproducible command script.
    pub async fn save_command_script(
        &self,
        ctx: &LogContext,
        script_content: &str,
    ) -> Result<(), std::io::Error> {
        if !self.config.enabled || !self.config.track_commands {
            return Ok(());
        }

        let command_file = self.command_file_path(ctx);
        fs::write(&command_file, script_content).await?;

        self.log_event(
            ctx,
            &format!("Command script saved to {}", command_file.display()),
        )
        .await?;

        Ok(())
    }

    /// Save tool uses.
    pub async fn save_tool_uses(
        &self,
        ctx: &LogContext,
        tool_uses: &[ToolUse],
    ) -> Result<(), std::io::Error> {
        if !self.config.enabled || !self.config.track_tool_uses || tool_uses.is_empty() {
            return Ok(());
        }

        let tools_file = self.tools_file_path(ctx);
        let tools_json = serde_json::to_string_pretty(tool_uses)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        fs::write(&tools_file, tools_json).await?;

        self.log_event(ctx, &format!("Captured {} tool uses", tool_uses.len()))
            .await?;

        Ok(())
    }

    /// Log completion with token usage and timing.
    pub async fn log_completion(
        &self,
        ctx: &LogContext,
        input_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
        estimated_cost: f64,
        execution_time_secs: f64,
    ) -> Result<(), std::io::Error> {
        if !self.config.enabled {
            return Ok(());
        }

        let completion_msg = format!(
            "Task completed successfully\n\
             Input tokens: {}\n\
             Output tokens: {}\n\
             Total tokens: {}\n\
             Estimated cost: ${:.6}\n\
             Execution time: {:.2}s\n\
             {}\n\
             Request processing completed for ID: {}",
            input_tokens,
            output_tokens,
            total_tokens,
            estimated_cost,
            execution_time_secs,
            "=".repeat(80),
            ctx.request_id
        );

        self.log_event(ctx, &completion_msg).await?;
        Ok(())
    }

    /// Log an error.
    pub async fn log_error(
        &self,
        ctx: &LogContext,
        error_message: &str,
    ) -> Result<(), std::io::Error> {
        if !self.config.enabled {
            return Ok(());
        }

        self.log_event(ctx, &format!("ERROR: {}", error_message))
            .await?;
        Ok(())
    }

    /// Create a preview of binary data for logging.
    fn create_preview(&self, data: &[u8]) -> String {
        let text = String::from_utf8_lossy(data);
        let max_len = self.config.max_preview_chars;

        if text.len() <= max_len {
            text.replace('\n', " ")
        } else {
            format!(
                "{}... (see full output in file)",
                text[..max_len].replace('\n', " ")
            )
        }
    }

    /// Check if logging is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if tool use tracking is enabled.
    pub fn is_tool_tracking_enabled(&self) -> bool {
        self.config.enabled && self.config.track_tool_uses
    }

    /// Check if command tracking is enabled.
    pub fn is_command_tracking_enabled(&self) -> bool {
        self.config.enabled && self.config.track_commands
    }
}

/// Builder for creating ProviderLogger with custom configuration.
pub struct ProviderLoggerBuilder {
    provider_name: String,
    config: ProviderLoggerConfig,
    logs_dir: Option<PathBuf>,
}

impl ProviderLoggerBuilder {
    /// Create a new builder for a provider.
    pub fn new(provider_name: impl Into<String>) -> Self {
        Self {
            provider_name: provider_name.into(),
            config: ProviderLoggerConfig::default(),
            logs_dir: None,
        }
    }

    /// Set whether logging is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Set whether to track tool uses.
    pub fn track_tool_uses(mut self, track: bool) -> Self {
        self.config.track_tool_uses = track;
        self
    }

    /// Set whether to track command scripts.
    pub fn track_commands(mut self, track: bool) -> Self {
        self.config.track_commands = track;
        self
    }

    /// Set maximum preview characters.
    pub fn max_preview_chars(mut self, max: usize) -> Self {
        self.config.max_preview_chars = max;
        self
    }

    /// Set logs directory.
    pub fn logs_dir(mut self, dir: PathBuf) -> Self {
        self.logs_dir = Some(dir);
        self
    }

    /// Build the logger.
    pub async fn build(self) -> Result<ProviderLogger, std::io::Error> {
        let logs_dir = self.logs_dir.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "logs_dir must be set")
        })?;

        ProviderLogger::new(self.provider_name, self.config, logs_dir).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_provider_logger_basic() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join("logs");

        let logger = ProviderLogger::new(
            "test-provider",
            ProviderLoggerConfig::default(),
            logs_dir.clone(),
        )
        .await
        .unwrap();

        let ctx = LogContext::new(Uuid::new_v4(), "test-model");

        logger
            .log_command_start(&ctx, "test command")
            .await
            .unwrap();
        logger.log_event(&ctx, "test event").await.unwrap();
        logger
            .save_stdout(&ctx, b"{\"result\": \"success\"}")
            .await
            .unwrap();
        logger
            .log_completion(&ctx, 100, 50, 150, 0.002, 1.5)
            .await
            .unwrap();

        // Verify files were created
        let log_file = logger.log_file_path(&ctx);
        let stdout_file = logger.stdout_file_path(&ctx);

        assert!(log_file.exists());
        assert!(stdout_file.exists());
    }

    #[tokio::test]
    async fn test_provider_logger_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join("logs");

        let config = ProviderLoggerConfig {
            enabled: false,
            ..Default::default()
        };

        let logger = ProviderLogger::new("test-provider", config, logs_dir.clone())
            .await
            .unwrap();

        let ctx = LogContext::new(Uuid::new_v4(), "test-model");

        // Should not create files when disabled
        logger
            .log_command_start(&ctx, "test command")
            .await
            .unwrap();

        let log_file = logger.log_file_path(&ctx);
        assert!(!log_file.exists());
    }

    #[tokio::test]
    async fn test_provider_logger_builder() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join("logs");

        let logger = ProviderLoggerBuilder::new("custom-provider")
            .enabled(true)
            .track_tool_uses(false)
            .track_commands(true)
            .max_preview_chars(200)
            .logs_dir(logs_dir)
            .build()
            .await
            .unwrap();

        assert!(logger.is_enabled());
        assert!(!logger.is_tool_tracking_enabled());
        assert!(logger.is_command_tracking_enabled());
    }
}
