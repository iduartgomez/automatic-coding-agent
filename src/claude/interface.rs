use crate::claude::{ContextManager, ErrorRecoveryManager, RateLimiter, UsageTracker, types::*};
use crate::task::types::{Task, TaskStatus};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug)]
pub struct ClaudeCodeInterface {
    config: ClaudeConfig,
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
    pub async fn new(config: ClaudeConfig) -> Result<Self, ClaudeError> {
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
    ) -> Result<TaskResponse, ClaudeError> {
        // Get or create a session
        let session = self.get_or_create_session().await?;

        // Track session start
        self.usage_tracker.start_session(session.id).await;

        // Execute request directly for now (TODO: add error recovery)
        let result = self.execute_request_internal(&session, &request).await;

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

        // Mock Claude response generation
        let response = self.generate_mock_response(request).await?;

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

    async fn generate_mock_response(
        &self,
        request: &TaskRequest,
    ) -> Result<TaskResponse, ClaudeError> {
        // Simulate processing time
        let processing_time = Duration::from_millis(100 + (rand::random::<u64>() % 900));
        tokio::time::sleep(processing_time).await;

        // Random error injection removed to prevent flaky tests

        let input_tokens = self.estimate_tokens(&request.description);
        let output_tokens = input_tokens / 2 + 50; // Mock output length
        let total_tokens = input_tokens + output_tokens;

        // Generate mock response based on task type
        let response_text = match request.task_type.as_str() {
            "code_generation" => format!(
                "I'll help you implement {}. Here's the solution:\n\n```rust\n// Mock implementation\nfn {}() {{\n    // TODO: Implement functionality\n    println!(\"Hello from mock implementation!\");\n}}\n```",
                request.description,
                request.description.replace(' ', "_").to_lowercase()
            ),
            "code_review" => format!(
                "I've reviewed the code. Here are my findings:\n\n1. The implementation looks good overall\n2. Consider adding error handling\n3. Unit tests would be beneficial\n\nFor: {}",
                request.description
            ),
            "debugging" => format!(
                "I've analyzed the issue: {}\n\nPossible causes:\n1. Check for null pointer exceptions\n2. Verify input validation\n3. Review error logs\n\nRecommended fix: Add proper error handling and logging.",
                request.description
            ),
            "refactoring" => format!(
                "Here's how I would refactor {}:\n\n1. Extract common functionality into utilities\n2. Improve naming conventions\n3. Add documentation\n4. Optimize performance",
                request.description
            ),
            _ => format!(
                "I understand you want help with: {}\n\nI'll analyze this and provide a comprehensive solution. This is a mock response for testing purposes.",
                request.description
            ),
        };

        let estimated_cost = self
            .usage_tracker
            .estimate_cost_for_tokens(input_tokens, output_tokens)
            .await;

        Ok(TaskResponse {
            task_id: request.id,
            response_text,
            tool_uses: vec![], // Mock: no tool uses for now
            token_usage: TokenUsage {
                input_tokens,
                output_tokens,
                total_tokens,
                estimated_cost,
            },
            execution_time: processing_time,
            model_used: "claude-3-mock".to_string(),
        })
    }

    fn estimate_tokens(&self, text: &str) -> u64 {
        // Simple token estimation: roughly 4 characters per token
        (text.len() as f64 / 4.0).ceil() as u64
    }

    async fn get_or_create_session(&self) -> Result<ClaudeSession, ClaudeError> {
        let mut pool = self.session_pool.lock().await;

        // Try to get an idle session first
        if let Some(session) = pool.idle_sessions.pop_front() {
            pool.active_sessions.insert(session.id, session.clone());
            return Ok(session);
        }

        // Check if we can create a new session
        if pool.active_sessions.len() >= self.config.session_config.max_concurrent_sessions as usize
        {
            return Err(ClaudeError::ServiceUnavailable(
                "Maximum number of concurrent sessions reached".to_string(),
            ));
        }

        // Create new session
        let session = ClaudeSession {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            last_used: Utc::now(),
            message_count: 0,
            is_busy: true,
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
        }
    }

    pub async fn process_task(&self, task: &Task) -> Result<Task, ClaudeError> {
        let request = TaskRequest {
            id: task.id,
            task_type: "task_processing".to_string(),
            description: task.description.clone(),
            context: std::collections::HashMap::new(),
            priority: TaskPriority::Medium, // Default priority for now
            estimated_tokens: Some(self.estimate_tokens(&task.description)),
        };

        let response = self.execute_task_request(request).await?;

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
