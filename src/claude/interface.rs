//! Claude Code CLI interface for task execution
//!
//! This module provides the interface to execute tasks using the `claude` CLI command
//! in headless mode. It includes:
//!
//! - Session management and pooling
//! - Rate limiting and usage tracking
//! - Context and conversation history management
//! - System message support via `--append-system-prompt`
//! - JSON output parsing from CLI responses
//!
//! ## CLI Command Structure
//!
//! ```bash
//! claude --print \
//!        --output-format json \
//!        --allowedTools Read,Write,Edit,Bash,Glob,Grep \
//!        --permission-mode acceptEdits \
//!        --append-system-prompt "System instructions..." \  # If provided
//!        --model sonnet \
//!        -- "User prompt"
//! ```
//!
//! ## Response Format
//!
//! The CLI returns JSON in the format:
//! ```json
//! {
//!   "type": "result",
//!   "result": "Actual Claude response content",
//!   "usage": { "inputTokens": 100, "outputTokens": 50 }
//! }
//! ```
//!
//! The `result` field contains the actual response text which is extracted and returned.

use crate::claude::{ContextManager, ErrorRecoveryManager, RateLimiter, UsageTracker, types::*};
use crate::env;
use crate::task::types::{Task, TaskStatus};
use chrono::{DateTime, Utc};
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug)]
pub struct ClaudeCodeInterface {
    config: ClaudeConfig,
    workspace_root: PathBuf,
    rate_limiter: Arc<RateLimiter>,
    context_manager: Arc<ContextManager>,
    usage_tracker: Arc<UsageTracker>,
    #[allow(dead_code)]
    error_recovery: Arc<ErrorRecoveryManager>,
    session_pool: Arc<Mutex<SessionPool>>,
}

#[derive(Debug)]
struct SessionPool {
    active_sessions: std::collections::HashMap<SessionId, ClaudeSession>,
    idle_sessions: std::collections::VecDeque<ClaudeSession>,
}

#[derive(Debug, Clone)]
struct ClaudeSession {
    pub id: SessionId,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub message_count: u32,
    pub is_busy: bool,
}

impl ClaudeCodeInterface {
    pub async fn new(config: ClaudeConfig, workspace_root: PathBuf) -> Result<Self, ClaudeError> {
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limits.clone()));
        let context_manager = Arc::new(ContextManager::new(config.context_config.clone()));
        let usage_tracker = Arc::new(UsageTracker::new(config.usage_tracking.clone()));
        let error_recovery = Arc::new(ErrorRecoveryManager::new(config.error_config.clone()));

        let session_pool = Arc::new(Mutex::new(SessionPool {
            active_sessions: std::collections::HashMap::new(),
            idle_sessions: std::collections::VecDeque::new(),
        }));

        Ok(Self {
            config,
            workspace_root,
            rate_limiter,
            context_manager,
            usage_tracker,
            error_recovery,
            session_pool,
        })
    }

    pub async fn execute_task_request(
        &self,
        request: TaskRequest,
        session_dir: Option<&std::path::Path>,
    ) -> Result<TaskResponse, ClaudeError> {
        // Get or create a session
        let session = self.get_or_create_session().await?;

        // Track session start
        self.usage_tracker.start_session(session.id).await;

        // Execute request directly for now (TODO: add error recovery)
        let result = self
            .execute_request_internal(&session, &request, session_dir)
            .await;

        // Update session state
        self.update_session_state(&session).await;

        // Record usage if successful
        if let Ok(ref response) = result {
            self.usage_tracker.record_usage(session.id, response).await;
        }

        result
    }

    async fn execute_request_internal(
        &self,
        session: &ClaudeSession,
        request: &TaskRequest,
        session_dir: Option<&std::path::Path>,
    ) -> Result<TaskResponse, ClaudeError> {
        // Apply rate limiting
        let _permit = self.rate_limiter.acquire_permit(request).await?;

        // Get conversation context
        let _context = self.context_manager.get_or_create_context(session.id).await;

        // Add user message to context
        let user_message = ClaudeMessage {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: request.description.clone(),
            timestamp: Utc::now(),
            token_count: Some(self.estimate_tokens(&request.description)),
            metadata: request.context.clone(),
        };

        self.context_manager
            .add_message(session.id, user_message)
            .await
            .map_err(|e| ClaudeError::Unknown(e.to_string()))?;

        // Execute real Claude Code request with session context
        let response = self
            .execute_claude_code_request(session.id, request, session_dir)
            .await?;

        // Add assistant response to context
        let assistant_message = ClaudeMessage {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: response.response_text.clone(),
            timestamp: Utc::now(),
            token_count: Some(response.token_usage.output_tokens),
            metadata: std::collections::HashMap::new(),
        };

        self.context_manager
            .add_message(session.id, assistant_message)
            .await
            .map_err(|e| ClaudeError::Unknown(e.to_string()))?;

        Ok(response)
    }

    async fn execute_claude_code_request(
        &self,
        session_id: SessionId,
        request: &TaskRequest,
        session_dir_override: Option<&std::path::Path>,
    ) -> Result<TaskResponse, ClaudeError> {
        let start_time = Instant::now();

        // Create log file for this request
        let log_path = self
            .create_subprocess_log_file(session_id, &request.id, session_dir_override)
            .await?;

        // Build contextual prompt with conversation history
        let contextual_prompt = self
            .build_contextual_prompt(session_id, &request.description)
            .await;

        const ALLOWED_TOOLS: &str =
            "Read,Write,Edit,Bash,Glob,Grep,MultiEdit,Task,TodoWrite,SlashCommand";

        // Determine output format based on tool tracking config
        let track_tool_uses = self.config.usage_tracking.track_tool_uses;
        let output_format = if track_tool_uses {
            "stream-json"
        } else {
            "json"
        };

        // Prepare Claude Code CLI command
        let mut command = Command::new("claude");
        command
            .arg("--print") // Non-interactive mode
            .arg("--output-format")
            .arg(output_format); // stream-json for tool tracking, json otherwise

        // stream-json requires --verbose
        if track_tool_uses {
            command.arg("--verbose");
        }

        command
            .arg("--allowedTools")
            .arg(ALLOWED_TOOLS) // Allow file operations
            .arg("--permission-mode")
            .arg("acceptEdits"); // Allow file modifications

        // Add system message if provided using --append-system-prompt
        let mut log_cmd = format!(
            "claude --print {}--output-format {} --allowedTools {ALLOWED_TOOLS} --permission-mode acceptEdits",
            if track_tool_uses { "--verbose " } else { "" },
            output_format,
        );
        if let Some(ref system_msg) = request.system_message {
            command.arg("--append-system-prompt").arg(system_msg);
            log_cmd.push_str(&format!(
                " --append-system-prompt {:?}",
                system_msg.chars().take(50).collect::<String>()
            ));
        }

        command
            .arg("--model")
            .arg("sonnet") // Use latest Sonnet model
            .arg("--") // Separate options from prompt
            .arg(&contextual_prompt); // The task description with conversation context

        // Configure stdio based on show_subprocess_output flag
        if self.config.show_subprocess_output {
            // Stream output to terminal in real-time
            command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null());
        } else {
            // Capture output silently
            command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null());
        }

        // Save exact command to .command.sh for reproducibility
        let command_file = log_path.with_extension("command.sh");
        let full_command = format!(
            "#!/bin/bash\n# Claude Code Command - Task ID: {}\n# Generated: {}\n\n{} --model sonnet -- {}\n",
            request.id,
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC"),
            log_cmd,
            shell_escape::escape(contextual_prompt.clone().into())
        );
        if let Err(e) = tokio::fs::write(&command_file, full_command).await {
            tracing::warn!("Failed to write command file: {}", e);
        }

        // Log command being executed
        self.log_subprocess_activity(
            &log_path,
            &format!(
                "[{}] Executing Claude Code command: {} --model sonnet -- {:?}",
                Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                log_cmd,
                request.description
            ),
        )
        .await;

        self.log_subprocess_activity(
            &log_path,
            &format!(
                "[{}] Task ID: {} | Description length: {} chars",
                Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                request.id,
                request.description.len()
            ),
        )
        .await;

        // Execute the command
        let output = if self.config.show_subprocess_output {
            tracing::info!("Streaming subprocess output to terminal...");
            // Stream output to terminal in real-time
            self.execute_with_streaming(command).await?
        } else {
            // Capture output silently
            command.output().await.map_err(|e| {
                // Log the error
                let error_msg = format!("Failed to execute claude command: {}", e);
                tokio::spawn({
                    let log_path = log_path.clone();
                    let error_msg = error_msg.clone();
                    async move {
                        if let Ok(mut f) = tokio::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&log_path)
                            .await
                        {
                            let _ = f
                                .write_all(
                                    format!(
                                        "[{}] ERROR: {}\n",
                                        Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                                        error_msg
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        }
                    }
                });
                ClaudeError::Unknown(error_msg)
            })?
        };

        let execution_time = start_time.elapsed();

        // Log execution completion
        self.log_subprocess_activity(&log_path, &format!(
            "[{}] Command completed in {:.2}s | Exit code: {} | Stdout: {} bytes | Stderr: {} bytes",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            execution_time.as_secs_f64(),
            output.status.code().unwrap_or(-1),
            output.stdout.len(),
            output.stderr.len()
        )).await;

        // Save full stdout/stderr to separate files for audit trail
        if !output.stdout.is_empty() {
            let stdout_file = log_path.with_extension("stdout.json");
            if let Err(e) = tokio::fs::write(&stdout_file, &output.stdout).await {
                tracing::warn!("Failed to write stdout file: {}", e);
            }

            // Extract and save tool uses if tool tracking is enabled
            if track_tool_uses
                && let Ok(tool_uses) = self.extract_tool_uses_from_stream(&output.stdout)
                && !tool_uses.is_empty()
            {
                let tools_file = log_path.with_extension("tools.json");
                if let Ok(tools_json) = serde_json::to_string_pretty(&tool_uses) {
                    if let Err(e) = tokio::fs::write(&tools_file, tools_json).await {
                        tracing::warn!("Failed to write tools file: {}", e);
                    } else {
                        tracing::info!("Captured {} tool uses", tool_uses.len());
                    }
                }
            }
        }
        if !output.stderr.is_empty() {
            let stderr_file = log_path.with_extension("stderr.txt");
            if let Err(e) = tokio::fs::write(&stderr_file, &output.stderr).await {
                tracing::warn!("Failed to write stderr file: {}", e);
            }
        }

        // Log stdout summary (first 500 chars for readability)
        if !output.stdout.is_empty() {
            let stdout_preview = String::from_utf8_lossy(&output.stdout);
            let preview = if stdout_preview.len() > 500 {
                format!(
                    "{}... (see full output in *.stdout.json)",
                    &stdout_preview[..500]
                )
            } else {
                stdout_preview.to_string()
            };
            self.log_subprocess_activity(
                &log_path,
                &format!(
                    "[{}] STDOUT: {}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    preview
                ),
            )
            .await;
        }

        // Log stderr summary (first 500 chars for readability)
        if !output.stderr.is_empty() {
            let stderr_preview = String::from_utf8_lossy(&output.stderr);
            let preview = if stderr_preview.len() > 500 {
                format!(
                    "{}... (see full output in *.stderr.txt)",
                    &stderr_preview[..500]
                )
            } else {
                stderr_preview.to_string()
            };
            self.log_subprocess_activity(
                &log_path,
                &format!(
                    "[{}] STDERR: {}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    preview
                ),
            )
            .await;
        }

        // Check if command succeeded
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_msg = format!(
                "Claude command failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            );

            self.log_subprocess_activity(
                &log_path,
                &format!(
                    "[{}] COMMAND FAILED: {}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    error_msg
                ),
            )
            .await;

            return Err(ClaudeError::Unknown(error_msg));
        }

        // Parse the output based on format
        let stdout = String::from_utf8_lossy(&output.stdout);
        let response_text = if stdout.trim().is_empty() {
            // If no JSON output, fall back to stderr which might contain the response
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim().is_empty() {
                "Task completed successfully".to_string()
            } else {
                stderr.to_string()
            }
        } else if track_tool_uses {
            // Parse stream-json format (JSONL)
            // Extract result from the final "result" type message
            self.parse_stream_json_result(&stdout)
                .unwrap_or_else(|_| "Task completed".to_string())
        } else {
            // Try to parse regular JSON response from Claude CLI (--output-format json)
            match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(json) => {
                    // Extract response text from JSON structure
                    // Claude CLI returns: {"result": "actual response as JSON string", ...}
                    if let Some(result_str) = json.get("result").and_then(|r| r.as_str()) {
                        result_str.to_string()
                    } else if let Some(response_str) = json.get("response").and_then(|r| r.as_str())
                    {
                        response_str.to_string()
                    } else if let Some(content_str) = json.get("content").and_then(|c| c.as_str()) {
                        content_str.to_string()
                    } else {
                        // Fall back to raw stdout if we can't find the response in expected fields
                        stdout.to_string()
                    }
                }
                Err(_) => stdout.to_string(),
            }
        };

        let input_tokens = self.estimate_tokens(&request.description);
        let output_tokens = self.estimate_tokens(&response_text);
        let total_tokens = input_tokens + output_tokens;

        let estimated_cost = self
            .usage_tracker
            .estimate_cost_for_tokens(input_tokens, output_tokens)
            .await;

        // Log successful completion
        self.log_subprocess_activity(&log_path, &format!(
            "[{}] Task completed successfully | Input tokens: {} | Output tokens: {} | Total tokens: {} | Estimated cost: ${:.6}",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            input_tokens,
            output_tokens,
            total_tokens,
            estimated_cost
        )).await;

        self.log_subprocess_activity(
            &log_path,
            &format!(
                "[{}] Response length: {} chars | Execution time: {:.2}s",
                Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                response_text.len(),
                execution_time.as_secs_f64()
            ),
        )
        .await;

        self.log_subprocess_activity(
            &log_path,
            &format!(
                "{}\nTask processing completed for ID: {}\n",
                "=".repeat(80),
                request.id
            ),
        )
        .await;

        Ok(TaskResponse {
            task_id: request.id,
            response_text,
            tool_uses: vec![], // TODO: Parse tool uses from JSON output
            token_usage: TokenUsage {
                input_tokens,
                output_tokens,
                total_tokens,
                estimated_cost,
            },
            execution_time,
            model_used: "sonnet".to_string(), // Latest Sonnet model
        })
    }

    fn estimate_tokens(&self, text: &str) -> u64 {
        // Simple token estimation: roughly 4 characters per token
        (text.len() as f64 / 4.0).ceil() as u64
    }

    /// Extract tool uses from stream-json format (JSONL)
    fn extract_tool_uses_from_stream(
        &self,
        stdout: &[u8],
    ) -> Result<Vec<serde_json::Value>, ClaudeError> {
        let stdout_str = String::from_utf8_lossy(stdout);
        let mut tool_uses = Vec::new();

        for line in stdout_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                // Look for assistant messages with tool_use content
                if json.get("type").and_then(|t| t.as_str()) == Some("assistant")
                    && let Some(content_array) = json
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_array())
                {
                    for item in content_array {
                        if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                            tool_uses.push(item.clone());
                        }
                    }
                }
            }
        }

        Ok(tool_uses)
    }

    /// Parse result from stream-json format (JSONL)
    fn parse_stream_json_result(&self, stdout: &str) -> Result<String, ClaudeError> {
        // Find the final "result" message
        for line in stdout.lines().rev() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line)
                && json.get("type").and_then(|t| t.as_str()) == Some("result")
                && let Some(result) = json.get("result").and_then(|r| r.as_str())
            {
                return Ok(result.to_string());
            }
        }

        Err(ClaudeError::Unknown(
            "No result found in stream-json output".to_string(),
        ))
    }

    /// Execute command with streaming output to terminal
    async fn execute_with_streaming(
        &self,
        mut command: Command,
    ) -> Result<std::process::Output, ClaudeError> {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

        let mut child = command.spawn().map_err(|e| {
            let error_msg = format!("Failed to spawn claude command: {}", e);
            ClaudeError::Unknown(error_msg)
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ClaudeError::Unknown("Failed to capture stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ClaudeError::Unknown("Failed to capture stderr".to_string()))?;

        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);

        let mut stdout_buffer = Vec::new();
        let mut stderr_buffer = Vec::new();
        let mut stdout_line = String::new();
        let mut stderr_line = String::new();

        // Stream output line by line
        loop {
            tokio::select! {
                result = stdout_reader.read_line(&mut stdout_line) => {
                    match result {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            // Print to terminal
                            print!("{}", stdout_line);
                            use std::io::Write;
                            let _ = std::io::stdout().flush();

                            // Save to buffer
                            stdout_buffer.extend_from_slice(stdout_line.as_bytes());
                            stdout_line.clear();
                        }
                        Err(e) => {
                            tracing::warn!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
                result = stderr_reader.read_line(&mut stderr_line) => {
                    match result {
                        Ok(0) => {}, // EOF on stderr
                        Ok(_) => {
                            // Print to terminal (stderr)
                            eprint!("{}", stderr_line);
                            use std::io::Write;
                            let _ = std::io::stderr().flush();

                            // Save to buffer
                            stderr_buffer.extend_from_slice(stderr_line.as_bytes());
                            stderr_line.clear();
                        }
                        Err(e) => {
                            tracing::warn!("Error reading stderr: {}", e);
                        }
                    }
                }
            }
        }

        // Read any remaining stderr
        let mut remaining_stderr = Vec::new();
        if let Ok(n) = stderr_reader.read_to_end(&mut remaining_stderr).await {
            if n > 0 {
                eprint!("{}", String::from_utf8_lossy(&remaining_stderr));
                stderr_buffer.extend_from_slice(&remaining_stderr);
            }
        }

        // Wait for process to complete
        let status = child.wait().await.map_err(|e| {
            ClaudeError::Unknown(format!("Failed to wait for child process: {}", e))
        })?;

        Ok(std::process::Output {
            status,
            stdout: stdout_buffer,
            stderr: stderr_buffer,
        })
    }

    async fn get_or_create_session(&self) -> Result<ClaudeSession, ClaudeError> {
        let mut pool = self.session_pool.lock().await;

        // For Claude Code CLI, we use a single shared session for all tasks
        // Each task creates its own subprocess, so we don't need separate sessions
        if let Some(session) = pool.active_sessions.values().next() {
            return Ok(session.clone());
        }

        // Create the single shared session
        let session = ClaudeSession {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            last_used: Utc::now(),
            message_count: 0,
            is_busy: false,
        };

        pool.active_sessions.insert(session.id, session.clone());
        Ok(session)
    }

    async fn update_session_state(&self, session: &ClaudeSession) {
        let mut pool = self.session_pool.lock().await;

        if let Some(active_session) = pool.active_sessions.get_mut(&session.id) {
            active_session.last_used = Utc::now();
            active_session.message_count += 1;
            active_session.is_busy = false;

            // Check if session should be moved to idle pool
            let idle_threshold = Duration::from_secs(300); // 5 minutes
            if Utc::now().signed_duration_since(active_session.last_used)
                > chrono::Duration::from_std(idle_threshold).unwrap_or_default()
            {
                let session = pool.active_sessions.remove(&session.id).unwrap();
                pool.idle_sessions.push_back(session);
            }
        }
    }

    pub async fn create_task_from_description(
        &self,
        description: &str,
        task_type: &str,
    ) -> TaskRequest {
        TaskRequest {
            id: Uuid::new_v4(),
            task_type: task_type.to_string(),
            description: description.to_string(),
            context: std::collections::HashMap::new(),
            priority: TaskPriority::Medium,
            estimated_tokens: Some(self.estimate_tokens(description)),
            system_message: None,
        }
    }

    pub async fn process_task(
        &self,
        task: &Task,
        session_dir: Option<&std::path::Path>,
    ) -> Result<Task, ClaudeError> {
        let request = TaskRequest {
            id: task.id,
            task_type: "task_processing".to_string(),
            description: task.description.clone(),
            context: std::collections::HashMap::new(),
            priority: TaskPriority::Medium, // Default priority for now
            estimated_tokens: Some(self.estimate_tokens(&task.description)),
            system_message: None,
        };

        let response = self.execute_task_request(request, session_dir).await?;

        // Create updated task with response
        let mut updated_task = task.clone();
        updated_task.status = TaskStatus::Completed {
            completed_at: Utc::now(),
            result: crate::task::types::TaskResult::Success {
                output: serde_json::json!({
                    "response": response.response_text,
                    "token_usage": response.token_usage,
                    "model_used": response.model_used
                }),
                files_created: Vec::new(),
                files_modified: Vec::new(),
                build_artifacts: Vec::new(),
            },
        };
        updated_task.updated_at = Utc::now();

        Ok(updated_task)
    }

    async fn create_subprocess_log_file(
        &self,
        session_id: SessionId,
        task_id: &Uuid,
        session_dir_override: Option<&std::path::Path>,
    ) -> Result<PathBuf, ClaudeError> {
        // Use provided session directory if available, otherwise fall back to workspace-based path
        let logs_dir = if let Some(session_dir) = session_dir_override {
            session_dir
                .join("logs")
                .join(env::session::CLAUDE_INTERACTIONS_DIR_NAME)
        } else {
            env::claude_interactions_dir_path(&self.workspace_root, &session_id.to_string())
        };

        tokio::fs::create_dir_all(&logs_dir)
            .await
            .map_err(|e| ClaudeError::Unknown(format!("Failed to create logs directory: {}", e)))?;

        let log_file = logs_dir.join(format!("claude-subprocess-{}.log", task_id));

        // Create the log file with initial header
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file)
            .await
            .map_err(|e| ClaudeError::Unknown(format!("Failed to create log file: {}", e)))?;

        file.write_all(
            format!(
                "Claude Code Subprocess Log - Task ID: {}\nStarted: {}\n{}\n",
                task_id,
                Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC"),
                "=".repeat(80)
            )
            .as_bytes(),
        )
        .await
        .map_err(|e| ClaudeError::Unknown(format!("Failed to write log header: {}", e)))?;

        Ok(log_file)
    }

    async fn log_subprocess_activity(&self, log_path: &PathBuf, message: &str) {
        if let Ok(mut file) = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await
        {
            let _ = file.write_all(format!("{}\n", message).as_bytes()).await;
        }
    }

    pub async fn get_interface_status(&self) -> ClaudeInterfaceStatus {
        let rate_status = self.rate_limiter.get_status().await;
        let usage_summary = self.usage_tracker.get_usage_summary(1).await;
        let context_stats = self.context_manager.get_context_stats().await;

        let pool = self.session_pool.lock().await;
        let session_stats = SessionPoolStats {
            active_sessions: pool.active_sessions.len(),
            idle_sessions: pool.idle_sessions.len(),
            max_sessions: self.config.session_config.max_concurrent_sessions,
        };

        ClaudeInterfaceStatus {
            rate_limiter: rate_status.clone(),
            usage_summary,
            context_stats,
            session_stats,
            is_healthy: rate_status.failure_count < 3,
        }
    }

    /// Build a contextual prompt that includes conversation history for better continuity
    async fn build_contextual_prompt(
        &self,
        session_id: SessionId,
        current_request: &str,
    ) -> String {
        // Get existing conversation context
        if let Some(context) = self.context_manager.get_context(session_id).await
            && !context.messages.is_empty()
        {
            // Format the conversation history
            let history = self.format_conversation_history(&context.messages);

            // Build contextual prompt with history and current request
            return format!(
                "Previous conversation context:\n{}\n\n--- Current Task ---\n{}",
                history, current_request
            );
        }

        // If no context exists, just return the current request
        current_request.to_string()
    }

    /// Format conversation messages into a readable conversation history
    fn format_conversation_history(&self, messages: &[ClaudeMessage]) -> String {
        let mut history = String::new();

        for (i, message) in messages.iter().enumerate() {
            let role = match message.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::System => "System",
            };

            let timestamp = message.timestamp.format("%H:%M:%S");

            // Truncate very long messages to keep context manageable
            let content = if message.content.len() > 1000 {
                format!("{}...[truncated]", &message.content[..1000])
            } else {
                message.content.clone()
            };

            history.push_str(&format!("[{}] {}: {}\n", timestamp, role, content));

            // Add separator between messages except for the last one
            if i < messages.len() - 1 {
                history.push('\n');
            }
        }

        history
    }
}

#[derive(Debug, Clone)]
pub struct ClaudeInterfaceStatus {
    pub rate_limiter: crate::claude::rate_limiter::RateLimiterStatus,
    pub usage_summary: crate::claude::usage_tracker::UsageSummary,
    pub context_stats: crate::claude::context_manager::ContextManagerStats,
    pub session_stats: SessionPoolStats,
    pub is_healthy: bool,
}

#[derive(Debug, Clone)]
pub struct SessionPoolStats {
    pub active_sessions: usize,
    pub idle_sessions: usize,
    pub max_sessions: u32,
}
