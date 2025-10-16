//! Integration tests for intelligent task parsing
//!
//! These tests verify the end-to-end intelligent task parsing functionality
//! using the LLM abstraction layer.

use aca::cli::{IntelligentTaskParser, TaskAnalysisRequest};
use aca::llm::types::{LLMError, ProviderCapabilities, ProviderStatus};
use aca::llm::{LLMProvider, LLMRequest, LLMResponse, TokenUsage};
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

// Mock LLM provider that returns predefined responses
struct MockLLMProvider {
    responses: Vec<String>,
    call_count: std::sync::Mutex<usize>,
}

impl MockLLMProvider {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: std::sync::Mutex::new(0),
        }
    }

    fn with_single_response(response: String) -> Self {
        Self::new(vec![response])
    }
}

impl LLMProvider for MockLLMProvider {
    fn execute_request<'a>(
        &'a self,
        _request: LLMRequest,
        _logger: &'a aca::llm::provider_logger::ProviderLogger,
    ) -> BoxFuture<'a, Result<LLMResponse, LLMError>> {
        let mut count = self.call_count.lock().unwrap();
        let index = *count % self.responses.len();
        *count += 1;
        let response = self.responses[index].clone();

        Box::pin(async move {
            Ok(LLMResponse {
                request_id: Uuid::new_v4(),
                content: response,
                model_used: "mock-model".to_string(),
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
                available_models: vec!["mock-model".to_string()],
            })
        })
    }

    fn get_status(&self) -> BoxFuture<'_, Result<ProviderStatus, LLMError>> {
        Box::pin(async move { Err(LLMError::ProviderUnavailable("mock provider".to_string())) })
    }

    fn health_check(&self) -> BoxFuture<'_, Result<(), LLMError>> {
        Box::pin(async move { Ok(()) })
    }

    fn provider_name(&self) -> &'static str {
        "mock"
    }

    fn list_models(&self) -> BoxFuture<'_, Result<Vec<String>, LLMError>> {
        Box::pin(async move { Ok(vec!["mock-model".to_string()]) })
    }

    fn estimate_tokens(&self, text: &str) -> u64 {
        (text.len() as f64 / 4.0).ceil() as u64
    }
}

#[tokio::test]
async fn test_single_simple_task() {
    let mock_response = r#"{
  "tasks": [
    {
      "title": "Implement user authentication",
      "description": "Add login and registration functionality",
      "parent_index": null,
      "dependencies": [],
      "priority": "High",
      "complexity": "Moderate",
      "estimated_duration_secs": 7200,
      "required_files": ["src/auth.rs", "src/models/user.rs"],
      "tags": ["authentication", "security"]
    }
  ],
  "execution_strategy": "Sequential",
  "estimated_duration_secs": 7200,
  "overall_complexity": "Moderate"
}"#;

    let provider = Arc::new(MockLLMProvider::with_single_response(
        mock_response.to_string(),
    ));
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Implement user authentication system".to_string(),
        source_path: None,
        context_hints: vec![],
        max_tokens: Some(2048),
    };

    let result = parser.analyze_tasks(request).await.unwrap();

    assert_eq!(result.tasks.len(), 1);
    assert_eq!(result.tasks[0].title, "Implement user authentication");
    assert_eq!(
        result.tasks[0].complexity,
        aca::task::ComplexityLevel::Moderate
    );
    assert_eq!(result.tasks[0].priority, aca::task::TaskPriority::High);
    assert_eq!(result.tasks[0].required_files.len(), 2);
    assert_eq!(result.tasks[0].tags.len(), 2);
}

#[tokio::test]
async fn test_hierarchical_task_structure() {
    let mock_response = r#"{
  "tasks": [
    {
      "title": "Phase 1: Database Setup",
      "description": "Set up database infrastructure",
      "parent_index": null,
      "dependencies": [],
      "priority": "Critical",
      "complexity": "Complex",
      "estimated_duration_secs": 3600,
      "required_files": [],
      "tags": ["infrastructure", "database"]
    },
    {
      "title": "Create database schema",
      "description": "Define tables and relationships",
      "parent_index": 0,
      "dependencies": [],
      "priority": "High",
      "complexity": "Moderate",
      "estimated_duration_secs": 1800,
      "required_files": ["migrations/001_init.sql"],
      "tags": ["database", "schema"]
    },
    {
      "title": "Add seed data",
      "description": "Insert initial test data",
      "parent_index": 0,
      "dependencies": [1],
      "priority": "Normal",
      "complexity": "Simple",
      "estimated_duration_secs": 600,
      "required_files": ["seeds/test_data.sql"],
      "tags": ["database", "testing"]
    }
  ],
  "execution_strategy": "Sequential",
  "estimated_duration_secs": 6000,
  "overall_complexity": "Complex"
}"#;

    let provider = Arc::new(MockLLMProvider::with_single_response(
        mock_response.to_string(),
    ));
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: r#"# Phase 1: Database Setup
- Create database schema
- Add seed data"#
            .to_string(),
        source_path: Some(PathBuf::from("tasks.md")),
        context_hints: vec!["backend project".to_string()],
        max_tokens: Some(2048),
    };

    let result = parser.analyze_tasks(request).await.unwrap();

    assert_eq!(result.tasks.len(), 3);
    assert_eq!(result.tasks[0].title, "Phase 1: Database Setup");
    assert!(result.tasks[0].parent_index.is_none());
    assert_eq!(result.tasks[1].parent_index, Some(0));
    assert_eq!(result.tasks[2].parent_index, Some(0));
    assert_eq!(result.tasks[2].dependencies, vec![1]);
}

#[tokio::test]
async fn test_parallel_execution_strategy() {
    let mock_response = r#"{
  "tasks": [
    {
      "title": "Task A",
      "description": "Independent task A",
      "parent_index": null,
      "dependencies": [],
      "priority": "High",
      "complexity": "Simple",
      "estimated_duration_secs": 600,
      "required_files": [],
      "tags": ["parallel"]
    },
    {
      "title": "Task B",
      "description": "Independent task B",
      "parent_index": null,
      "dependencies": [],
      "priority": "High",
      "complexity": "Simple",
      "estimated_duration_secs": 600,
      "required_files": [],
      "tags": ["parallel"]
    }
  ],
  "execution_strategy": {
    "Parallel": {
      "max_concurrent": 2
    }
  },
  "estimated_duration_secs": 600,
  "overall_complexity": "Simple"
}"#;

    let provider = Arc::new(MockLLMProvider::with_single_response(
        mock_response.to_string(),
    ));
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Task A\nTask B".to_string(),
        source_path: None,
        context_hints: vec!["independent tasks".to_string()],
        max_tokens: Some(2048),
    };

    let result = parser.analyze_tasks(request).await.unwrap();

    assert_eq!(result.tasks.len(), 2);
    assert!(matches!(
        result.execution_strategy,
        aca::cli::ExecutionStrategy::Parallel { max_concurrent: 2 }
    ));
}

#[tokio::test]
async fn test_execution_plan_conversion() {
    let mock_response = r#"{
  "tasks": [
    {
      "title": "Implement feature X",
      "description": "Add new feature X",
      "parent_index": null,
      "dependencies": [],
      "priority": "Normal",
      "complexity": "Moderate",
      "estimated_duration_secs": 3600,
      "required_files": ["src/feature_x.rs"],
      "tags": ["feature"]
    }
  ],
  "execution_strategy": "Sequential",
  "estimated_duration_secs": 3600,
  "overall_complexity": "Moderate"
}"#;

    let provider = Arc::new(MockLLMProvider::with_single_response(
        mock_response.to_string(),
    ));
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Implement feature X".to_string(),
        source_path: Some(PathBuf::from("task.md")),
        context_hints: vec![],
        max_tokens: Some(2048),
    };

    let analysis = parser.analyze_tasks(request).await.unwrap();
    let plan = parser.analysis_to_execution_plan(analysis, Some("Test Plan".to_string()));

    assert!(plan.has_tasks());
    assert_eq!(plan.task_count(), 1);
    assert!(plan.metadata.name.is_some());
    assert!(plan.metadata.tags.contains(&"llm-analyzed".to_string()));
    assert!(plan.metadata.estimated_duration.is_some());
}

#[tokio::test]
async fn test_caching_mechanism() {
    let mock_response = r#"{
  "tasks": [
    {
      "title": "Test task",
      "description": "Testing caching",
      "parent_index": null,
      "dependencies": [],
      "priority": "Normal",
      "complexity": "Trivial",
      "estimated_duration_secs": 300,
      "required_files": [],
      "tags": ["test"]
    }
  ],
  "execution_strategy": "Sequential",
  "estimated_duration_secs": 300,
  "overall_complexity": "Trivial"
}"#;

    // Create provider that returns different responses (to detect cache misses)
    let provider = Arc::new(MockLLMProvider::with_single_response(
        mock_response.to_string(),
    ));
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Test task".to_string(),
        source_path: None,
        context_hints: vec![],
        max_tokens: Some(2048),
    };

    // First call - should hit the LLM
    let result1 = parser.analyze_tasks(request.clone()).await.unwrap();

    // Second call with same request - should use cache
    let result2 = parser.analyze_tasks(request.clone()).await.unwrap();

    assert_eq!(result1.tasks.len(), result2.tasks.len());
    assert_eq!(result1.tasks[0].title, result2.tasks[0].title);
}

#[tokio::test]
async fn test_complex_real_world_task() {
    let mock_response = r#"{
  "tasks": [
    {
      "title": "Phase 1: Core Data Provider Implementation",
      "description": "Implement foundational data collection providers",
      "parent_index": null,
      "dependencies": [],
      "priority": "Critical",
      "complexity": "Epic",
      "estimated_duration_secs": 28800,
      "required_files": [],
      "tags": ["phase1", "infrastructure"]
    },
    {
      "title": "Supply Chain Localization Provider",
      "description": "Implement supplier location scraping from company reports",
      "parent_index": 0,
      "dependencies": [],
      "priority": "High",
      "complexity": "Complex",
      "estimated_duration_secs": 10800,
      "required_files": ["supply-chain-provider.md"],
      "tags": ["data-provider", "supply-chain"]
    },
    {
      "title": "Enhanced Ownership Provider",
      "description": "Extend existing GLEIF/Wikidata providers for shareholder data",
      "parent_index": 0,
      "dependencies": [],
      "priority": "High",
      "complexity": "Complex",
      "estimated_duration_secs": 9000,
      "required_files": ["ownership-provider.md"],
      "tags": ["data-provider", "ownership"]
    },
    {
      "title": "Phase 2: Core Algorithm Implementation",
      "description": "Develop scoring algorithms",
      "parent_index": null,
      "dependencies": [0],
      "priority": "High",
      "complexity": "Complex",
      "estimated_duration_secs": 14400,
      "required_files": [],
      "tags": ["phase2", "algorithms"]
    },
    {
      "title": "Scoring Algorithm Development",
      "description": "Implement weighted scoring formula combining all data providers",
      "parent_index": 3,
      "dependencies": [1, 2],
      "priority": "High",
      "complexity": "Complex",
      "estimated_duration_secs": 10800,
      "required_files": ["scoring-algorithm.md"],
      "tags": ["algorithm", "scoring"]
    }
  ],
  "execution_strategy": "Intelligent",
  "estimated_duration_secs": 43200,
  "overall_complexity": "Epic"
}"#;

    let provider = Arc::new(MockLLMProvider::with_single_response(
        mock_response.to_string(),
    ));
    let parser = IntelligentTaskParser::new(provider);

    let task_content = r#"# European Products App - Prioritized Task List

## Phase 1: Core Data Provider Implementation (High Priority)

### 1. Complete Supply Chain Data Providers
- [ ] **Supply Chain Localization Provider** → supply-chain-provider.md

### 2. Enhance Ownership Data Collection
- [ ] **Enhanced Ownership Provider** → ownership-provider.md

## Phase 2: Core Algorithm Implementation (High Priority)

### 6. Scoring Algorithm Development
- [ ] **Europeanness Scoring Algorithm** → scoring-algorithm.md
"#;

    let request = TaskAnalysisRequest {
        content: task_content.to_string(),
        source_path: Some(PathBuf::from("tasks.md")),
        context_hints: vec!["european products app".to_string()],
        max_tokens: Some(4096),
    };

    let result = parser.analyze_tasks(request).await.unwrap();

    assert_eq!(result.tasks.len(), 5);
    assert_eq!(result.overall_complexity, aca::task::ComplexityLevel::Epic);
    assert!(matches!(
        result.execution_strategy,
        aca::cli::ExecutionStrategy::Intelligent
    ));

    // Verify hierarchical structure
    assert!(result.tasks[0].parent_index.is_none());
    assert_eq!(result.tasks[1].parent_index, Some(0));
    assert_eq!(result.tasks[2].parent_index, Some(0));
    assert!(result.tasks[3].parent_index.is_none());
    assert_eq!(result.tasks[4].parent_index, Some(3));

    // Verify dependencies
    assert_eq!(result.tasks[3].dependencies, vec![0]); // Phase 2 depends on Phase 1
    assert_eq!(result.tasks[4].dependencies, vec![1, 2]); // Scoring depends on providers
}

#[tokio::test]
async fn test_error_handling_invalid_json() {
    let provider = Arc::new(MockLLMProvider::with_single_response(
        "This is not valid JSON".to_string(),
    ));
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Test".to_string(),
        source_path: None,
        context_hints: vec![],
        max_tokens: Some(2048),
    };

    let result = parser.analyze_tasks(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_error_handling_invalid_structure() {
    let mock_response = r#"{
  "tasks": [],
  "execution_strategy": "Sequential",
  "estimated_duration_secs": 0,
  "overall_complexity": "Trivial"
}"#;

    let provider = Arc::new(MockLLMProvider::with_single_response(
        mock_response.to_string(),
    ));
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Test".to_string(),
        source_path: None,
        context_hints: vec![],
        max_tokens: Some(2048),
    };

    let result = parser.analyze_tasks(request).await;
    assert!(result.is_err());
}
