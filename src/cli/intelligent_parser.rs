//! Intelligent task parsing using LLM for decomposition and analysis
//!
//! This module provides LLM-powered task parsing that can:
//! - Analyze complex task descriptions and break them into structured hierarchies
//! - Automatically follow and read markdown file references (e.g., `[spec](detail.md)`)
//! - Extract ALL subtasks and phases with technical detail preservation
//! - Identify task dependencies and map them to TaskIds
//! - Assign priorities and complexity estimates
//! - Generate optimal execution strategies
//!
//! ## Features
//!
//! ### Markdown File Reference Resolution
//! The parser automatically detects and reads linked markdown files:
//! ```markdown
//! ## Task 1: Setup Database
//! → Details: [database-setup.md](database-setup.md)
//! ```
//! The content of `database-setup.md` is automatically included in the analysis.
//!
//! ### Dependency Mapping
//! Dependencies are extracted from LLM analysis as indices, then mapped to deterministic
//! TaskIds using UUID v5 (name-based). This ensures:
//! - Consistent task IDs across runs
//! - Proper dependency graph construction
//! - Support for complex task relationships
//!
//! ### System Message Support
//! System messages are passed via Claude CLI's `--append-system-prompt` flag for
//! clean separation of instructions from user content.
//!
//! ## Performance
//!
//! - **Detail Preservation**: Typical 6 high-level tasks → 42+ detailed subtasks
//! - **Reference Resolution**: Supports multiple linked files with cycle prevention
//! - **Caching**: Responses cached by content hash for improved performance
//!
//! Unlike the naive `TaskLoader`, this parser understands semantic meaning and context.

use crate::llm::{LLMProvider, LLMRequest};
use crate::task::{
    ComplexityLevel, ContextRequirements, ExecutionPlan, TaskId, TaskMetadata, TaskPriority,
    TaskSpec,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum IntelligentParserError {
    #[error("LLM provider error: {0}")]
    LLMError(#[from] crate::llm::types::LLMError),

    #[error("Failed to parse LLM response: {0}")]
    ParseError(String),

    #[error("Invalid task structure: {0}")]
    InvalidStructure(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Request for intelligent task analysis
#[derive(Debug, Clone)]
pub struct TaskAnalysisRequest {
    /// Raw content to analyze (markdown, plain text, etc.)
    pub content: String,
    /// Optional source file path for context
    pub source_path: Option<PathBuf>,
    /// Additional context hints to guide analysis
    pub context_hints: Vec<String>,
    /// Maximum number of tokens to use for analysis
    pub max_tokens: Option<u64>,
}

/// Result of intelligent task analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysisResult {
    /// List of analyzed tasks with hierarchy information
    pub tasks: Vec<AnalyzedTask>,
    /// Suggested execution strategy
    pub execution_strategy: ExecutionStrategy,
    /// Estimated total duration in seconds
    pub estimated_duration_secs: Option<u64>,
    /// Overall complexity assessment
    pub overall_complexity: ComplexityLevel,
}

/// A task that has been analyzed by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedTask {
    /// Task title (concise)
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Parent task index (None for root tasks)
    pub parent_index: Option<usize>,
    /// Indices of tasks this depends on
    pub dependencies: Vec<usize>,
    /// Task priority
    pub priority: TaskPriority,
    /// Estimated complexity
    pub complexity: ComplexityLevel,
    /// Estimated duration in seconds
    pub estimated_duration_secs: Option<u64>,
    /// Required files for context
    pub required_files: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Suggested execution strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStrategy {
    /// Execute tasks sequentially in order
    Sequential,
    /// Execute tasks in parallel where dependencies allow
    Parallel { max_concurrent: usize },
    /// Use intelligent scheduling based on priorities and dependencies
    Intelligent,
}

/// Intelligent task parser that uses LLM for analysis
pub struct IntelligentTaskParser {
    llm_provider: Arc<dyn LLMProvider>,
    enable_caching: bool,
    cache: std::sync::Mutex<HashMap<String, TaskAnalysisResult>>,
}

impl IntelligentTaskParser {
    /// Create a new intelligent task parser
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            llm_provider,
            enable_caching: true,
            cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Create parser with caching disabled
    pub fn without_caching(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            llm_provider,
            enable_caching: false,
            cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Analyze a task request and return structured analysis
    pub async fn analyze_tasks(
        &self,
        request: TaskAnalysisRequest,
    ) -> Result<TaskAnalysisResult, IntelligentParserError> {
        // Check cache first
        if self.enable_caching {
            let cache_key = self.generate_cache_key(&request);
            if let Ok(cache) = self.cache.lock()
                && let Some(cached_result) = cache.get(&cache_key)
            {
                debug!("Using cached task analysis for key: {}", cache_key);
                return Ok(cached_result.clone());
            }
        }

        info!(
            "Analyzing tasks using LLM (source: {:?})",
            request.source_path
        );

        // Build the LLM prompt
        let prompt = self.build_analysis_prompt(&request);
        debug!("Generated analysis prompt ({} chars)", prompt.len());

        // Estimate tokens
        let estimated_tokens = self.llm_provider.estimate_tokens(&prompt);
        debug!("Estimated prompt tokens: {}", estimated_tokens);

        // Create LLM request
        let llm_request = LLMRequest {
            id: Uuid::new_v4(),
            prompt,
            context: self.build_context(&request),
            max_tokens: request.max_tokens.or(Some(4096)),
            temperature: Some(0.3), // Lower temperature for more consistent parsing
            model_preference: None,
            system_message: Some(self.get_system_message()),
        };

        // Execute LLM request
        let response = self.llm_provider.execute_request(llm_request).await?;

        info!(
            "LLM analysis complete (tokens: input={}, output={})",
            response.token_usage.input_tokens, response.token_usage.output_tokens
        );

        // Parse the LLM response
        let analysis_result = self.parse_llm_response(&response.content)?;

        // Validate the result
        self.validate_analysis(&analysis_result)?;

        // Cache the result
        if self.enable_caching {
            let cache_key = self.generate_cache_key(&request);
            if let Ok(mut cache) = self.cache.lock() {
                cache.insert(cache_key, analysis_result.clone());
            }
        }

        Ok(analysis_result)
    }

    /// Convert analysis result to execution plan
    pub fn analysis_to_execution_plan(
        &self,
        analysis: TaskAnalysisResult,
        source_name: Option<String>,
    ) -> ExecutionPlan {
        // First pass: Create all task specs and collect their IDs
        let mut task_specs: Vec<TaskSpec> = Vec::new();
        let mut task_ids: Vec<TaskId> = Vec::new();

        // Use a namespace UUID for generating deterministic task IDs
        let namespace = uuid::Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(); // DNS namespace

        for analyzed_task in &analysis.tasks {
            let spec = self.analyzed_task_to_spec(analyzed_task.clone());
            // Generate a deterministic TaskId using UUID v5 (name-based)
            let task_name = format!("llm-task-{}", analyzed_task.title);
            let task_id = uuid::Uuid::new_v5(&namespace, task_name.as_bytes());
            task_ids.push(task_id);
            task_specs.push(spec);
        }

        // Second pass: Map dependency indices to TaskIds
        for (i, analyzed_task) in analysis.tasks.iter().enumerate() {
            for dep_index in &analyzed_task.dependencies {
                if let Some(dep_task_id) = task_ids.get(*dep_index) {
                    task_specs[i].dependencies.push(*dep_task_id);
                }
            }
        }

        let execution_mode = match analysis.execution_strategy {
            ExecutionStrategy::Sequential => crate::task::ExecutionMode::Sequential,
            ExecutionStrategy::Parallel { max_concurrent } => {
                crate::task::ExecutionMode::Parallel {
                    max_concurrent: Some(max_concurrent),
                }
            }
            ExecutionStrategy::Intelligent => crate::task::ExecutionMode::Intelligent,
        };

        let plan_name = source_name.unwrap_or_else(|| "Intelligent Task Analysis".to_string());
        let task_count = task_specs.len();

        let mut plan = ExecutionPlan::new()
            .with_tasks(task_specs)
            .with_execution_mode(execution_mode)
            .with_metadata(
                plan_name,
                format!(
                    "Execution plan generated from intelligent LLM analysis ({} tasks)",
                    task_count
                ),
            )
            .with_tags(vec![
                "llm-analyzed".to_string(),
                "intelligent-parser".to_string(),
            ]);

        if let Some(duration_secs) = analysis.estimated_duration_secs {
            plan = plan.with_estimated_duration(
                chrono::Duration::from_std(std::time::Duration::from_secs(duration_secs))
                    .unwrap_or(chrono::Duration::minutes(5)),
            );
        }

        plan
    }

    /// Parse task request from file and analyze it
    pub async fn parse_file(
        &self,
        path: PathBuf,
        context_hints: Vec<String>,
    ) -> Result<ExecutionPlan, IntelligentParserError> {
        let content = std::fs::read_to_string(&path)?;

        // Extract and resolve markdown file references
        let content_with_refs = self.resolve_file_references(&path, &content)?;

        let request = TaskAnalysisRequest {
            content: content_with_refs,
            source_path: Some(path.clone()),
            context_hints,
            max_tokens: Some(8192), // Increased for larger content with references
        };

        let analysis = self.analyze_tasks(request).await?;

        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| format!("Tasks from {}", s));

        Ok(self.analysis_to_execution_plan(analysis, source_name))
    }

    // Private helper methods

    /// Resolve markdown file references and include their content
    fn resolve_file_references(
        &self,
        base_path: &std::path::Path,
        content: &str,
    ) -> Result<String, IntelligentParserError> {
        use regex::Regex;

        let mut resolved_content = content.to_string();

        // Regex to match markdown links: [text](path.md)
        let link_regex = Regex::new(r"\[([^\]]+)\]\(([^\)]+\.md)\)").unwrap();

        let base_dir = base_path.parent().unwrap_or(std::path::Path::new("."));
        let mut referenced_files = Vec::new();

        // Extract all markdown file references
        for cap in link_regex.captures_iter(content) {
            if let Some(file_path) = cap.get(2) {
                let ref_path = base_dir.join(file_path.as_str());
                if ref_path.exists() && !referenced_files.contains(&ref_path) {
                    referenced_files.push(ref_path);
                }
            }
        }

        // Read and append referenced files (limit depth to prevent cycles)
        if !referenced_files.is_empty() {
            resolved_content.push_str("\n\n---\n\n# Referenced Detail Files\n\n");

            for (idx, ref_path) in referenced_files.iter().enumerate() {
                if let Ok(ref_content) = std::fs::read_to_string(ref_path) {
                    let file_name = ref_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    resolved_content.push_str(&format!(
                        "## Referenced File {}: {}\n\n{}\n\n",
                        idx + 1,
                        file_name,
                        ref_content
                    ));

                    debug!(
                        "Resolved reference: {} ({} bytes)",
                        file_name,
                        ref_content.len()
                    );
                }
            }
        }

        Ok(resolved_content)
    }

    fn generate_cache_key(&self, request: &TaskAnalysisRequest) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        request.content.hash(&mut hasher);
        request.context_hints.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn build_analysis_prompt(&self, request: &TaskAnalysisRequest) -> String {
        let mut prompt = String::new();

        prompt.push_str("# Task Analysis Request\n\n");
        prompt.push_str(
            "Analyze the following task description and provide a structured breakdown with ALL subtasks and phases.\n\n",
        );

        if let Some(ref path) = request.source_path {
            prompt.push_str(&format!("**Source**: {}\n\n", path.display()));
        }

        if !request.context_hints.is_empty() {
            prompt.push_str("**Context Hints**:\n");
            for hint in &request.context_hints {
                prompt.push_str(&format!("- {}\n", hint));
            }
            prompt.push('\n');
        }

        prompt.push_str("**Task Content**:\n```\n");
        prompt.push_str(&request.content);
        prompt.push_str("\n```\n\n");

        prompt.push_str("CRITICAL: Extract ALL subtasks, phases, and implementation details from the content above.\n");
        prompt.push_str(
            "Create separate task entries for each distinct phase or subtask mentioned.\n",
        );
        prompt.push_str("Use parent_index to represent hierarchical relationships.\n");
        prompt.push_str("Include ALL technical details, requirements, and success criteria in task descriptions.\n\n");

        prompt.push_str("IMPORTANT: Respond with ONLY a valid JSON object (no markdown code blocks, no explanations, no additional text).\n\n");
        prompt.push_str("Expected JSON schema:\n");
        prompt.push_str(&self.get_response_schema());
        prompt.push_str(
            "\n\nYour response must be pure JSON starting with '{' and ending with '}'\n",
        );

        prompt
    }

    fn build_context(&self, request: &TaskAnalysisRequest) -> HashMap<String, String> {
        let mut context = HashMap::new();

        if let Some(ref path) = request.source_path {
            context.insert("source_path".to_string(), path.display().to_string());
        }

        if !request.context_hints.is_empty() {
            context.insert(
                "context_hints".to_string(),
                request.context_hints.join(", "),
            );
        }

        context
    }

    fn get_system_message(&self) -> String {
        r#"You are an expert task decomposition assistant. Your role is to analyze task descriptions and break them into structured, executable units.

CRITICAL INSTRUCTIONS:
1. **Preserve ALL details and nuance** - Do NOT summarize or collapse detailed task information
2. **Create hierarchical structures** - If tasks have subtasks or phases, represent them as separate tasks with parent_index
3. **Extract ALL subtasks** - If a task mentions "Phase 1: X, Phase 2: Y", create separate tasks for each phase
4. **Keep technical details** - Include implementation details, technical requirements, success criteria in descriptions
5. **Identify nested structures** - Tasks referencing detailed spec files should have child tasks for major components

When analyzing tasks:
1. Identify distinct tasks and ALL subtasks with clear boundaries
2. Create parent-child relationships for hierarchical task structures (use parent_index)
3. Determine logical dependencies and execution order
4. Assign appropriate priorities based on impact and urgency
5. Estimate complexity and duration realistically for EACH subtask
6. Extract ALL file references and context requirements mentioned
7. Preserve technical specifications, metrics, and success criteria
8. Suggest optimal execution strategy (sequential/parallel/intelligent)

Examples of proper decomposition:
- If task mentions "Phase 1: Setup (Week 1-2), Phase 2: Implementation (Week 3-4)", create 2 separate tasks
- If task has "Task 1.1: X, Task 1.2: Y, Task 2.1: Z", create 3 separate tasks with parent relationships
- If task references detailed files, include those files in required_files

Always respond with valid JSON matching the provided schema. Be precise, thorough, and preserve all detail."#
            .to_string()
    }

    fn get_response_schema(&self) -> String {
        r#"{
  "tasks": [
    {
      "title": "Task title (concise, < 80 chars)",
      "description": "Detailed task description",
      "parent_index": null or number,
      "dependencies": [0, 1],
      "priority": "Critical" | "High" | "Normal" | "Low" | "Background",
      "complexity": "Trivial" | "Simple" | "Moderate" | "Complex" | "Epic",
      "estimated_duration_secs": number or null,
      "required_files": ["path/to/file.rs"],
      "tags": ["tag1", "tag2"]
    }
  ],
  "execution_strategy": {
    "Sequential" | { "Parallel": { "max_concurrent": number } } | "Intelligent"
  },
  "estimated_duration_secs": number or null,
  "overall_complexity": "Trivial" | "Simple" | "Moderate" | "Complex" | "Epic"
}"#
        .to_string()
    }

    fn parse_llm_response(
        &self,
        content: &str,
    ) -> Result<TaskAnalysisResult, IntelligentParserError> {
        // Try to extract JSON from markdown code blocks if present
        let json_content = if let Some(start) = content.find("```json") {
            let json_start = start + 7;
            if let Some(end) = content[json_start..].find("```") {
                content[json_start..json_start + end].trim()
            } else {
                content
            }
        } else if let Some(start) = content.find('{') {
            // Find the first { and try to parse from there
            &content[start..]
        } else {
            content
        };

        // Try to parse directly first
        match serde_json::from_str::<TaskAnalysisResult>(json_content) {
            Ok(result) => Ok(result),
            Err(first_err) => {
                // If that fails, try to parse as a JSON-encoded string (double-encoded JSON)
                // This handles cases where the response is a JSON string containing escaped JSON
                if let Ok(unescaped) = serde_json::from_str::<String>(json_content) {
                    serde_json::from_str(&unescaped).map_err(|e| {
                        IntelligentParserError::ParseError(format!(
                            "Failed to parse unescaped JSON response: {}. Original error: {}. Content: {}",
                            e,
                            first_err,
                            json_content.chars().take(200).collect::<String>()
                        ))
                    })
                } else {
                    Err(IntelligentParserError::ParseError(format!(
                        "Failed to parse JSON response: {}. Content: {}",
                        first_err,
                        json_content.chars().take(200).collect::<String>()
                    )))
                }
            }
        }
    }

    fn validate_analysis(
        &self,
        analysis: &TaskAnalysisResult,
    ) -> Result<(), IntelligentParserError> {
        if analysis.tasks.is_empty() {
            return Err(IntelligentParserError::InvalidStructure(
                "Analysis contains no tasks".to_string(),
            ));
        }

        // Validate task indices
        let task_count = analysis.tasks.len();
        for (i, task) in analysis.tasks.iter().enumerate() {
            if let Some(parent) = task.parent_index
                && parent >= task_count
            {
                return Err(IntelligentParserError::InvalidStructure(format!(
                    "Task {} references invalid parent index {}",
                    i, parent
                )));
            }

            for &dep in &task.dependencies {
                if dep >= task_count {
                    return Err(IntelligentParserError::InvalidStructure(format!(
                        "Task {} references invalid dependency index {}",
                        i, dep
                    )));
                }
            }
        }

        Ok(())
    }

    fn analyzed_task_to_spec(&self, task: AnalyzedTask) -> TaskSpec {
        let mut context_requirements = ContextRequirements::default();

        // Convert required files to PathBuf
        for file_str in task.required_files {
            context_requirements
                .required_files
                .push(PathBuf::from(file_str));
        }

        let estimated_duration = task.estimated_duration_secs.map(|secs| {
            chrono::Duration::from_std(std::time::Duration::from_secs(secs))
                .unwrap_or(chrono::Duration::minutes(5))
        });

        TaskSpec {
            title: task.title,
            description: task.description,
            // Dependencies will be populated in analysis_to_execution_plan()
            // after all tasks are created and TaskIds are assigned
            dependencies: Vec::new(),
            metadata: TaskMetadata {
                priority: task.priority,
                estimated_complexity: Some(task.complexity),
                estimated_duration,
                repository_refs: Vec::new(),
                file_refs: Vec::new(),
                tags: task.tags,
                context_requirements,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::TokenUsage;
    use crate::llm::types::{LLMError, LLMResponse, ProviderCapabilities, ProviderStatus};
    use futures::future::BoxFuture;
    use std::time::Duration;

    // Mock LLM provider for testing
    struct MockLLMProvider {
        response: String,
    }

    impl MockLLMProvider {
        fn new(response: String) -> Self {
            Self { response }
        }
    }

    impl LLMProvider for MockLLMProvider {
        fn execute_request(
            &self,
            _request: LLMRequest,
        ) -> BoxFuture<'_, Result<crate::llm::LLMResponse, LLMError>> {
            let response = self.response.clone();
            Box::pin(async move {
                Ok(LLMResponse {
                    request_id: Uuid::new_v4(),
                    content: response,
                    model_used: "mock".to_string(),
                    token_usage: TokenUsage {
                        input_tokens: 100,
                        output_tokens: 200,
                        total_tokens: 300,
                        estimated_cost: 0.01,
                    },
                    execution_time: Duration::from_millis(100),
                    provider_metadata: HashMap::new(),
                })
            })
        }

        fn get_capabilities(&self) -> BoxFuture<'_, Result<ProviderCapabilities, LLMError>> {
            Box::pin(async move {
                Ok(ProviderCapabilities {
                    supports_streaming: false,
                    supports_function_calling: true,
                    supports_vision: false,
                    max_context_tokens: 100000,
                    available_models: vec!["mock".to_string()],
                })
            })
        }

        fn get_status(&self) -> BoxFuture<'_, Result<ProviderStatus, LLMError>> {
            Box::pin(async move { Err(LLMError::ProviderUnavailable("mock".to_string())) })
        }

        fn health_check(&self) -> BoxFuture<'_, Result<(), LLMError>> {
            Box::pin(async move { Ok(()) })
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn list_models(&self) -> BoxFuture<'_, Result<Vec<String>, LLMError>> {
            Box::pin(async move { Ok(vec!["mock".to_string()]) })
        }

        fn estimate_tokens(&self, text: &str) -> u64 {
            (text.len() as f64 / 4.0).ceil() as u64
        }
    }

    #[tokio::test]
    async fn test_simple_task_analysis() {
        let mock_response = r#"
{
  "tasks": [
    {
      "title": "Implement feature X",
      "description": "Add new feature X to the system",
      "parent_index": null,
      "dependencies": [],
      "priority": "High",
      "complexity": "Moderate",
      "estimated_duration_secs": 3600,
      "required_files": ["src/main.rs"],
      "tags": ["feature", "high-priority"]
    }
  ],
  "execution_strategy": "Sequential",
  "estimated_duration_secs": 3600,
  "overall_complexity": "Moderate"
}
"#;

        let provider = Arc::new(MockLLMProvider::new(mock_response.to_string()));
        let parser = IntelligentTaskParser::new(provider);

        let request = TaskAnalysisRequest {
            content: "Implement feature X".to_string(),
            source_path: None,
            context_hints: vec![],
            max_tokens: Some(2048),
        };

        let result = parser.analyze_tasks(request).await.unwrap();

        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].title, "Implement feature X");
        assert_eq!(result.tasks[0].priority, TaskPriority::High);
        assert_eq!(result.execution_strategy, ExecutionStrategy::Sequential);
    }

    #[tokio::test]
    async fn test_hierarchical_task_analysis() {
        let mock_response = r#"
{
  "tasks": [
    {
      "title": "Phase 1: Setup",
      "description": "Initial setup phase",
      "parent_index": null,
      "dependencies": [],
      "priority": "High",
      "complexity": "Simple",
      "estimated_duration_secs": 1800,
      "required_files": [],
      "tags": ["setup", "phase1"]
    },
    {
      "title": "Initialize database",
      "description": "Set up database schema",
      "parent_index": 0,
      "dependencies": [],
      "priority": "High",
      "complexity": "Moderate",
      "estimated_duration_secs": 900,
      "required_files": ["migrations/001_init.sql"],
      "tags": ["database", "setup"]
    }
  ],
  "execution_strategy": "Sequential",
  "estimated_duration_secs": 2700,
  "overall_complexity": "Moderate"
}
"#;

        let provider = Arc::new(MockLLMProvider::new(mock_response.to_string()));
        let parser = IntelligentTaskParser::new(provider);

        let request = TaskAnalysisRequest {
            content: "# Phase 1: Setup\n- Initialize database".to_string(),
            source_path: Some(PathBuf::from("tasks.md")),
            context_hints: vec!["database project".to_string()],
            max_tokens: Some(2048),
        };

        let result = parser.analyze_tasks(request).await.unwrap();

        assert_eq!(result.tasks.len(), 2);
        assert_eq!(result.tasks[1].parent_index, Some(0));
    }
}
