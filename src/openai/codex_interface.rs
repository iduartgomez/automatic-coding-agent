use crate::env;
use crate::openai::rate_limiter::{OpenAIRateLimiter, RateLimiterStatus};
use crate::openai::types::{
    OpenAIConfig, OpenAIError, OpenAITaskRequest, OpenAITaskResponse, RatePermit, TokenUsage,
};
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, warn};
use uuid::Uuid;
use which::which;

/// Interface that executes the Codex CLI in headless mode.
#[derive(Debug)]
pub struct OpenAICodexInterface {
    config: OpenAIConfig,
    rate_limiter: Arc<OpenAIRateLimiter>,
}

impl OpenAICodexInterface {
    pub async fn new(config: OpenAIConfig) -> Result<Self, OpenAIError> {
        if which(&config.cli_path).is_err() {
            return Err(OpenAIError::CliUnavailable(config.cli_path.clone()));
        }

        let rate_limiter = Arc::new(OpenAIRateLimiter::new(config.rate_limits.clone()));

        Ok(Self {
            config,
            rate_limiter,
        })
    }

    pub async fn execute_task_request(
        &self,
        request: OpenAITaskRequest,
        session_dir: Option<&Path>,
    ) -> Result<OpenAITaskResponse, OpenAIError> {
        let permit = self.rate_limiter.acquire_permit(&request).await?;
        let log_paths = if self.config.logging.enable_interaction_logs {
            self.prepare_log_paths(session_dir, request.id).await
        } else {
            None
        };

        if let Some(ref paths) = log_paths {
            self.append_log(
                paths,
                &format!(
                    "[{}] Starting Codex exec | model={} | tokens≈{}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    request.model,
                    request.estimated_tokens
                ),
            )
            .await;
        }

        let composed_prompt = self.compose_prompt(&request);
        let start = Instant::now();

        let mut skip_model_flag = false;
        let response = loop {
            let output = match self
                .run_codex_exec(&request, &composed_prompt, skip_model_flag)
                .await
            {
                Ok(output) => output,
                Err(err) => {
                    self.rate_limiter.record_failure().await;
                    return Err(err);
                }
            };

            let execution_time = start.elapsed();

            let parsed = match self.parse_codex_output(&output.stdout) {
                Ok(parsed) => parsed,
                Err(err) => {
                    self.rate_limiter.record_failure().await;
                    return Err(err);
                }
            };

            if let Some(ref paths) = log_paths {
                if let Err(e) = fs::write(&paths.stdout_file, &output.stdout).await {
                    warn!("Failed to persist Codex stdout: {}", e);
                }
                if !output.stderr.is_empty()
                    && let Err(e) = fs::write(&paths.stderr_file, &output.stderr).await
                {
                    warn!("Failed to persist Codex stderr: {}", e);
                }
            }

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let message = stderr.trim().to_string();
                if !skip_model_flag && message.contains("Unsupported model") {
                    info!(
                        "Codex CLI reported unsupported model; retrying without explicit --model flag"
                    );
                    skip_model_flag = true;
                    continue;
                }
                if message.contains("login") {
                    self.rate_limiter.record_failure().await;
                    return Err(OpenAIError::Authentication(message));
                }
                self.rate_limiter.record_failure().await;
                return Err(OpenAIError::CliFailed(message));
            }

            let response = self.build_response(&request, permit.clone(), parsed, execution_time)?;
            self.rate_limiter.record_success().await;
            break response;
        };

        if let Some(ref paths) = log_paths {
            let status = self.rate_limiter.get_status().await;
            self.append_log(
                paths,
                &format!(
                    "[{}] Completed in {:.2}s | preview=\"{}\" | remaining req={} tok={}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    response.execution_time.as_secs_f64(),
                    self.preview(&response.response_text),
                    status.available_requests,
                    status.available_tokens
                ),
            )
            .await;
        }

        info!(
            "Codex CLI request {} completed in {:.2?}",
            response.task_id, response.execution_time
        );

        Ok(response)
    }

    pub async fn get_interface_status(&self) -> RateLimiterStatus {
        self.rate_limiter.get_status().await
    }

    fn compose_prompt(&self, request: &OpenAITaskRequest) -> String {
        let mut segments = Vec::new();

        if let Some(system) = request.system_message.as_ref()
            && !system.trim().is_empty()
        {
            segments.push(format!("System instructions:\n{}\n", system.trim()));
        }

        if !request.metadata.is_empty() {
            let mut ctx = String::from("Context:\n");
            for (key, value) in &request.metadata {
                ctx.push_str(&format!("• {}: {}\n", key, value.trim()));
            }
            ctx.push('\n');
            segments.push(ctx);
        }

        segments.push(request.prompt.clone());
        segments.join("\n")
    }

    async fn run_codex_exec(
        &self,
        request: &OpenAITaskRequest,
        prompt: &str,
        skip_model_flag: bool,
    ) -> Result<std::process::Output, OpenAIError> {
        let mut cmd = Command::new(&self.config.cli_path);
        cmd.arg("exec")
            .arg("--json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if !skip_model_flag {
            cmd.arg("--model").arg(&request.model);
        }

        if self.config.allow_outside_git {
            cmd.arg("--skip-git-repo-check");
        }

        if let Some(profile) = &self.config.profile {
            cmd.arg("--profile").arg(profile);
        }

        if !self.config.extra_args.is_empty() {
            cmd.args(&self.config.extra_args);
        }

        cmd.arg("-")
            .stdin(Stdio::piped())
            .current_dir(&self.config.working_dir);

        let mut child = cmd
            .spawn()
            .map_err(|e| OpenAIError::CliFailed(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            let prompt_bytes = prompt.as_bytes();
            stdin
                .write_all(prompt_bytes)
                .await
                .map_err(|e| OpenAIError::CliFailed(e.to_string()))?;
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| OpenAIError::CliFailed(e.to_string()))?;

        Ok(output)
    }

    fn parse_codex_output(&self, stdout: &[u8]) -> Result<CodexParsedOutput, OpenAIError> {
        let text = String::from_utf8_lossy(stdout);
        let mut last_agent_message: Option<String> = None;
        let mut finish_reason: Option<String> = None;
        let mut usage = TokenUsage::default();
        let mut failure_reason: Option<String> = None;

        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let value: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                OpenAIError::Serialization(format!("Failed to parse Codex JSON line: {}", e))
            })?;

            if let Some(event_type) = value.get("type").and_then(|t| t.as_str()) {
                match event_type {
                    "item.completed" => {
                        if let Some(item) = value.get("item")
                            && item
                                .get("type")
                                .and_then(|t| t.as_str())
                                .map(|t| t == "agent_message")
                                .unwrap_or(false)
                            && let Some(text) = item.get("text").and_then(|t| t.as_str())
                        {
                            last_agent_message = Some(text.to_string());
                        }
                    }
                    "turn.completed" => {
                        if let Some(usage_value) = value.get("usage") {
                            usage.prompt_tokens = usage_value
                                .get("input_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            usage.cached_prompt_tokens = usage_value
                                .get("cached_input_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            usage.completion_tokens = usage_value
                                .get("output_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            usage.total_tokens = usage.prompt_tokens + usage.completion_tokens;
                        }
                    }
                    "run.completed" => {
                        finish_reason = value
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| Some("completed".to_string()));
                    }
                    "error" => {
                        if let Some(message) = value.get("message").and_then(|v| v.as_str()) {
                            failure_reason = Some(message.to_string());
                        }
                    }
                    "turn.failed" | "run.failed" => {
                        finish_reason = Some("failed".to_string());
                        if let Some(error) = value
                            .get("error")
                            .and_then(|v| v.get("message"))
                            .and_then(|v| v.as_str())
                        {
                            failure_reason = Some(error.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(response_text) = last_agent_message {
            return Ok(CodexParsedOutput {
                response_text,
                finish_reason,
                usage,
            });
        }

        if let Some(reason) = failure_reason {
            return Err(OpenAIError::CliFailed(reason));
        }

        Err(OpenAIError::CliFailed(
            "Codex CLI did not return an agent message".to_string(),
        ))
    }

    fn build_response(
        &self,
        request: &OpenAITaskRequest,
        permit: RatePermit,
        parsed: CodexParsedOutput,
        elapsed: Duration,
    ) -> Result<OpenAITaskResponse, OpenAIError> {
        let mut usage = parsed.usage;
        if usage.prompt_tokens == 0 {
            usage.prompt_tokens = permit.tokens_consumed;
            usage.total_tokens = usage.prompt_tokens + usage.completion_tokens;
        }

        let response = OpenAITaskResponse {
            task_id: request.id,
            response_text: parsed.response_text,
            token_usage: usage,
            execution_time: elapsed,
            model_used: request.model.clone(),
            finish_reason: parsed.finish_reason,
        };

        Ok(response)
    }

    async fn prepare_log_paths(
        &self,
        session_dir: Option<&Path>,
        request_id: Uuid,
    ) -> Option<LogPaths> {
        let base_dir = if let Some(dir) = session_dir {
            dir.join(env::session::OPENAI_INTERACTIONS_DIR_NAME)
        } else {
            env::openai_interactions_dir_path(&self.config.working_dir, "global")
        };

        if let Err(e) = fs::create_dir_all(&base_dir).await {
            warn!(
                "Failed to create Codex interaction directory {}: {}",
                base_dir.display(),
                e
            );
            return None;
        }

        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3f");
        let base = base_dir.join(format!("{}-{}", timestamp, request_id));

        Some(LogPaths {
            log_file: base.with_extension("log"),
            stdout_file: base.with_extension("stdout.jsonl"),
            stderr_file: base.with_extension("stderr.txt"),
        })
    }

    async fn append_log(&self, paths: &LogPaths, message: &str) {
        if let Err(e) = self.write_line(&paths.log_file, message).await {
            warn!(
                "Failed to append Codex interaction log {}: {}",
                paths.log_file.display(),
                e
            );
        }
    }

    async fn write_line(&self, file_path: &Path, content: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .await?;
        file.write_all(content.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }

    fn preview(&self, text: &str) -> String {
        let max_len = self.config.logging.max_preview_chars;
        if text.len() <= max_len {
            text.replace('\n', " ")
        } else {
            format!("{}...", text[..max_len].replace('\n', " "))
        }
    }
}

#[derive(Debug)]
struct LogPaths {
    log_file: PathBuf,
    stdout_file: PathBuf,
    stderr_file: PathBuf,
}

#[derive(Debug)]
struct CodexParsedOutput {
    response_text: String,
    finish_reason: Option<String>,
    usage: TokenUsage,
}
