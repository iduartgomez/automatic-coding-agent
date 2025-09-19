use crate::claude::types::{ClaudeMessage, ConversationContext, ContextConfig, SessionId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ContextManager {
    config: ContextConfig,
    contexts: Arc<Mutex<HashMap<SessionId, ConversationContext>>>,
}

#[derive(Debug, Clone)]
pub struct OptimizedContext {
    pub messages: Vec<ClaudeMessage>,
    pub total_tokens: u64,
    pub compression_applied: bool,
    pub messages_removed: u32,
    pub compression_ratio: f64,
}

impl ContextManager {
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            contexts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_or_create_context(&self, session_id: SessionId) -> ConversationContext {
        let mut contexts = self.contexts.lock().await;

        contexts.entry(session_id).or_insert_with(|| ConversationContext {
            session_id,
            messages: Vec::new(),
            total_tokens: 0,
            last_activity: chrono::Utc::now(),
            context_summary: None,
        }).clone()
    }

    pub async fn add_message(&self, session_id: SessionId, message: ClaudeMessage) -> Result<(), anyhow::Error> {
        let mut contexts = self.contexts.lock().await;

        if let Some(context) = contexts.get_mut(&session_id) {
            context.messages.push(message.clone());
            if let Some(token_count) = message.token_count {
                context.total_tokens += token_count;
            }
            context.last_activity = chrono::Utc::now();

            // Check if we need to optimize the context
            if context.messages.len() > self.config.max_history_length as usize ||
               context.total_tokens > (self.config.compression_threshold * 100000.0) as u64 {
                self.optimize_context_internal(context).await?;
            }
        }

        Ok(())
    }

    pub async fn optimize_context(&self, session_id: SessionId) -> Result<OptimizedContext, anyhow::Error> {
        let mut contexts = self.contexts.lock().await;

        if let Some(context) = contexts.get_mut(&session_id) {
            self.optimize_context_internal(context).await
        } else {
            Err(anyhow::anyhow!("Context not found for session {}", session_id))
        }
    }

    async fn optimize_context_internal(&self, context: &mut ConversationContext) -> Result<OptimizedContext, anyhow::Error> {
        let original_message_count = context.messages.len();
        let original_token_count = context.total_tokens;

        // Calculate relevance scores for each message
        let relevance_scores = self.calculate_relevance_scores(&context.messages).await?;

        // Sort messages by relevance (keep most relevant ones)
        let mut message_relevance: Vec<(usize, f64)> = relevance_scores.into_iter().enumerate().collect();
        message_relevance.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Keep the most recent messages and most relevant ones
        let keep_count = (self.config.max_history_length as usize).min(context.messages.len());
        let mut keep_indices = std::collections::HashSet::new();

        // Always keep the last few messages
        let recent_keep = 5.min(context.messages.len());
        for i in (context.messages.len() - recent_keep)..context.messages.len() {
            keep_indices.insert(i);
        }

        // Keep the most relevant messages up to our limit
        for (idx, relevance) in message_relevance.iter().take(keep_count) {
            if *relevance >= self.config.relevance_threshold {
                keep_indices.insert(*idx);
            }
        }

        // Filter messages
        let mut new_messages = Vec::new();
        let mut new_token_count = 0;

        for (idx, message) in context.messages.iter().enumerate() {
            if keep_indices.contains(&idx) {
                new_messages.push(message.clone());
                if let Some(tokens) = message.token_count {
                    new_token_count += tokens;
                }
            }
        }

        // Update context
        context.messages = new_messages;
        context.total_tokens = new_token_count;

        let messages_removed = original_message_count - context.messages.len();
        let compression_ratio = if original_token_count > 0 {
            1.0 - (new_token_count as f64 / original_token_count as f64)
        } else {
            0.0
        };

        Ok(OptimizedContext {
            messages: context.messages.clone(),
            total_tokens: new_token_count,
            compression_applied: messages_removed > 0,
            messages_removed: messages_removed as u32,
            compression_ratio,
        })
    }

    async fn calculate_relevance_scores(&self, messages: &[ClaudeMessage]) -> Result<Vec<f64>, anyhow::Error> {
        let mut scores = Vec::with_capacity(messages.len());

        for (idx, message) in messages.iter().enumerate() {
            let mut score = 0.0;

            // Temporal relevance (more recent = higher score)
            let age_factor = (messages.len() - idx) as f64 / messages.len() as f64;
            score += age_factor * 0.3;

            // Content length factor (longer messages might be more important)
            let length_factor = (message.content.len() as f64 / 1000.0).min(1.0);
            score += length_factor * 0.2;

            // Role factor (assistant messages might be more important to keep)
            let role_factor = match message.role {
                crate::claude::types::MessageRole::Assistant => 0.4,
                crate::claude::types::MessageRole::User => 0.3,
                crate::claude::types::MessageRole::System => 0.5,
            };
            score += role_factor * 0.3;

            // Keyword relevance (simple heuristic)
            let keyword_factor = self.calculate_keyword_relevance(&message.content);
            score += keyword_factor * 0.2;

            scores.push(score.min(1.0));
        }

        Ok(scores)
    }

    fn calculate_keyword_relevance(&self, content: &str) -> f64 {
        let important_keywords = [
            "error", "warning", "issue", "problem", "solution", "fix", "implement",
            "create", "build", "test", "debug", "critical", "important", "task",
            "function", "class", "method", "variable", "module", "package"
        ];

        let content_lower = content.to_lowercase();
        let mut keyword_count = 0;

        for keyword in &important_keywords {
            if content_lower.contains(keyword) {
                keyword_count += 1;
            }
        }

        (keyword_count as f64 / important_keywords.len() as f64).min(1.0)
    }

    pub async fn get_context(&self, session_id: SessionId) -> Option<ConversationContext> {
        let contexts = self.contexts.lock().await;
        contexts.get(&session_id).cloned()
    }

    pub async fn clear_context(&self, session_id: SessionId) -> Result<(), anyhow::Error> {
        let mut contexts = self.contexts.lock().await;
        contexts.remove(&session_id);
        Ok(())
    }

    pub async fn get_context_stats(&self) -> ContextManagerStats {
        let contexts = self.contexts.lock().await;

        let total_contexts = contexts.len();
        let total_messages: usize = contexts.values().map(|c| c.messages.len()).sum();
        let total_tokens: u64 = contexts.values().map(|c| c.total_tokens).sum();
        let avg_messages_per_context = if total_contexts > 0 {
            total_messages as f64 / total_contexts as f64
        } else {
            0.0
        };

        ContextManagerStats {
            total_contexts,
            total_messages,
            total_tokens,
            avg_messages_per_context,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextManagerStats {
    pub total_contexts: usize,
    pub total_messages: usize,
    pub total_tokens: u64,
    pub avg_messages_per_context: f64,
}