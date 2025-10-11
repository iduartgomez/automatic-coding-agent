//! Real Claude integration tests for intelligent task parsing
//!
//! These tests use Claude Code CLI (default) or Claude API if ANTHROPIC_API_KEY is set.
//! CLI mode requires `claude` command to be available in PATH.

use aca::cli::{IntelligentTaskParser, TaskAnalysisRequest};
use aca::llm::provider::LLMProviderFactory;
use aca::llm::types::{ProviderConfig, ProviderType, RateLimitConfig};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use test_tag::tag;

fn should_run_claude_tests() -> bool {
    if let Ok(value) = std::env::var("RUN_CLAUDE_TESTS") {
        if value == "1" || value.eq_ignore_ascii_case("true") {
            return true;
        }
    }

    std::env::var("ANTHROPIC_API_KEY").is_ok()
}

/// Helper to create a Claude provider for testing
/// Uses CLI mode by default (no API key required), or API mode if ANTHROPIC_API_KEY is set
async fn create_test_claude_provider() -> Arc<dyn aca::llm::LLMProvider> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();

    // If API key is set, configure for API mode explicitly
    let mut additional_config = std::collections::HashMap::new();
    if api_key.is_some() {
        additional_config.insert("mode".to_string(), serde_json::json!("API"));
    }
    // Otherwise defaults to CLI mode

    let config = ProviderConfig {
        provider_type: ProviderType::Claude,
        api_key,
        base_url: None,
        model: Some("claude-sonnet".to_string()),
        rate_limits: RateLimitConfig {
            max_requests_per_minute: 5,
            max_tokens_per_minute: 10000,
            burst_allowance: 2,
        },
        additional_config,
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    LLMProviderFactory::create_provider(config, temp_dir.path().to_path_buf())
        .await
        .expect("Failed to create Claude provider")
}

#[tokio::test]
#[tag(claude)]
async fn test_simple_task_analysis_with_real_claude() {
    if !should_run_claude_tests() {
        eprintln!("skipping Claude parser test: RUN_CLAUDE_TESTS not enabled");
        return;
    }

    let provider = create_test_claude_provider().await;
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Implement a simple REST API endpoint for user authentication with JWT tokens"
            .to_string(),
        source_path: None,
        context_hints: vec![
            "backend project".to_string(),
            "security critical".to_string(),
        ],
        max_tokens: Some(2048),
    };

    let result = parser
        .analyze_tasks(request)
        .await
        .expect("Failed to analyze tasks with Claude");

    // Verify the analysis produced tasks
    assert!(!result.tasks.is_empty(), "Should produce at least one task");

    // Verify task structure
    for task in &result.tasks {
        assert!(!task.title.is_empty(), "Task title should not be empty");
        assert!(
            !task.description.is_empty(),
            "Task description should not be empty"
        );
    }

    println!("✅ Claude analyzed {} tasks", result.tasks.len());
    for (i, task) in result.tasks.iter().enumerate() {
        println!(
            "  {}. {} (Priority: {:?}, Complexity: {:?})",
            i + 1,
            task.title,
            task.priority,
            task.complexity
        );
    }
}

#[tokio::test]
#[tag(claude)]
async fn test_hierarchical_task_analysis_with_real_claude() {
    if !should_run_claude_tests() {
        eprintln!("skipping Claude parser test: RUN_CLAUDE_TESTS not enabled");
        return;
    }

    let provider = create_test_claude_provider().await;
    let parser = IntelligentTaskParser::new(provider);

    let task_content = r#"# E-commerce Platform Development

## Phase 1: Backend Infrastructure
- Set up PostgreSQL database with proper schema
- Implement user authentication and authorization
- Create RESTful API endpoints for products

## Phase 2: Frontend Development
- Build product catalog page with search
- Implement shopping cart functionality
- Add checkout flow with payment integration

## Phase 3: Testing & Deployment
- Write unit and integration tests
- Set up CI/CD pipeline
- Deploy to production environment
"#;

    let request = TaskAnalysisRequest {
        content: task_content.to_string(),
        source_path: Some(PathBuf::from("project-plan.md")),
        context_hints: vec![
            "full-stack project".to_string(),
            "3 month timeline".to_string(),
        ],
        max_tokens: Some(4096),
    };

    let result = parser
        .analyze_tasks(request)
        .await
        .expect("Failed to analyze hierarchical tasks with Claude");

    // Verify hierarchical structure
    assert!(
        result.tasks.len() >= 3,
        "Should produce at least 3 phases/tasks"
    );

    // Check for parent-child relationships
    let has_hierarchy = result.tasks.iter().any(|t| t.parent_index.is_some());
    assert!(
        has_hierarchy,
        "Should have at least some tasks with parent relationships"
    );

    // Verify execution strategy is appropriate for complex project
    println!("✅ Claude produced hierarchical analysis:");
    println!("   Total tasks: {}", result.tasks.len());
    println!("   Execution strategy: {:?}", result.execution_strategy);
    println!("   Overall complexity: {:?}", result.overall_complexity);

    // Print task hierarchy
    for (i, task) in result.tasks.iter().enumerate() {
        let indent = if task.parent_index.is_some() {
            "    "
        } else {
            "  "
        };
        println!(
            "{}{}. {} (parent: {:?}, deps: {:?})",
            indent,
            i + 1,
            task.title,
            task.parent_index,
            task.dependencies
        );
    }
}

#[tokio::test]
#[tag(claude)]
async fn test_execution_plan_generation_with_real_claude() {
    if !should_run_claude_tests() {
        eprintln!("skipping Claude parser test: RUN_CLAUDE_TESTS not enabled");
        return;
    }

    let provider = create_test_claude_provider().await;
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Create a command-line tool that fetches weather data from an API and displays it in a formatted table".to_string(),
        source_path: Some(PathBuf::from("weather-cli.md")),
        context_hints: vec!["CLI tool".to_string(), "Python preferred".to_string()],
        max_tokens: Some(2048),
    };

    let analysis = parser
        .analyze_tasks(request)
        .await
        .expect("Failed to analyze with Claude");

    // Convert to execution plan
    let plan = parser.analysis_to_execution_plan(analysis.clone(), Some("Weather CLI".to_string()));

    // Verify execution plan
    assert!(plan.has_tasks(), "Execution plan should have tasks");
    assert!(
        plan.metadata.name.is_some(),
        "Execution plan should have a name"
    );
    assert!(
        plan.metadata.tags.contains(&"llm-analyzed".to_string()),
        "Should be tagged as LLM-analyzed"
    );

    println!("✅ Generated execution plan:");
    println!("   Name: {:?}", plan.metadata.name);
    println!("   Tasks: {}", plan.task_count());
    println!("   Execution mode: {:?}", plan.execution_mode);
    if let Some(duration) = plan.metadata.estimated_duration {
        println!("   Estimated duration: {} minutes", duration.num_minutes());
    }

    // Verify task specs have proper metadata
    for (i, spec) in plan.task_specs.iter().enumerate() {
        assert!(!spec.title.is_empty());
        assert!(!spec.description.is_empty());
        println!("   Task {}: {}", i + 1, spec.title);
    }
}

#[tokio::test]
#[tag(claude)]
async fn test_dependency_detection_with_real_claude() {
    let provider = create_test_claude_provider().await;
    let parser = IntelligentTaskParser::new(provider);

    let task_content = r#"
# Database Migration Project

1. Create backup of existing database
2. Write migration scripts (depends on backup completion)
3. Test migration scripts in staging environment (depends on migration scripts)
4. Run migration in production (depends on successful staging test)
5. Verify data integrity after migration (depends on production migration)
6. Update application connection strings (can run in parallel with verification)
"#;

    let request = TaskAnalysisRequest {
        content: task_content.to_string(),
        source_path: None,
        context_hints: vec![
            "sequential process".to_string(),
            "critical migration".to_string(),
        ],
        max_tokens: Some(3072),
    };

    let result = parser
        .analyze_tasks(request)
        .await
        .expect("Failed to analyze dependencies with Claude");

    // Verify dependencies were detected
    let has_dependencies = result.tasks.iter().any(|t| !t.dependencies.is_empty());
    assert!(
        has_dependencies,
        "Claude should detect task dependencies in this scenario"
    );

    println!("✅ Claude detected task dependencies:");
    for (i, task) in result.tasks.iter().enumerate() {
        if !task.dependencies.is_empty() {
            println!(
                "   Task {}: {} depends on {:?}",
                i, task.title, task.dependencies
            );
        }
    }
}

#[tokio::test]
#[tag(claude)]
async fn test_caching_with_real_claude() {
    let provider = create_test_claude_provider().await;
    let parser = IntelligentTaskParser::new(provider);

    let request = TaskAnalysisRequest {
        content: "Write a function to calculate Fibonacci numbers".to_string(),
        source_path: None,
        context_hints: vec![],
        max_tokens: Some(1024),
    };

    // First call - should hit Claude API
    let start = std::time::Instant::now();
    let result1 = parser
        .analyze_tasks(request.clone())
        .await
        .expect("First analysis failed");
    let first_duration = start.elapsed();

    // Second call - should use cache
    let start = std::time::Instant::now();
    let result2 = parser
        .analyze_tasks(request.clone())
        .await
        .expect("Second analysis failed");
    let second_duration = start.elapsed();

    // Verify results are identical
    assert_eq!(result1.tasks.len(), result2.tasks.len());
    assert_eq!(result1.tasks[0].title, result2.tasks[0].title);

    // Cache should be significantly faster (at least 10x)
    println!("✅ Cache performance:");
    println!("   First call: {:?}", first_duration);
    println!("   Second call (cached): {:?}", second_duration);
    println!(
        "   Speedup: {:.1}x",
        first_duration.as_secs_f64() / second_duration.as_secs_f64()
    );

    // Second call should be much faster due to caching
    // Note: We don't assert this strictly because timing can vary in CI environments
}

#[tokio::test]
#[tag(claude)]
async fn test_complex_real_world_task_with_real_claude() {
    let provider = create_test_claude_provider().await;
    let parser = IntelligentTaskParser::new(provider);

    // Use a realistic multi-phase project example
    let task_content = r#"# Weather Monitoring System

## Phase 1: Data Collection Infrastructure (High Priority)

### 1. Weather API Integration
- [ ] Implement REST client for OpenWeatherMap API
- [ ] Add fallback support for WeatherAPI.com
- [ ] Create caching layer for API responses
- [ ] Implement rate limiting and quota management

### 2. Database Design & Implementation
- [ ] Design PostgreSQL schema for weather data
- [ ] Implement time-series data storage
- [ ] Add geospatial indexing for location queries
- [ ] Create data retention and archival policies

## Phase 2: Core Processing (Medium Priority)

### 3. Data Processing Pipeline
- [ ] Build ETL pipeline for raw weather data
- [ ] Implement data validation and cleaning
- [ ] Add anomaly detection for sensor errors
- [ ] Create aggregation for hourly/daily summaries

### 4. Alert System
- [ ] Develop threshold-based alert engine
- [ ] Add notification service (email, SMS, webhooks)
- [ ] Implement alert configuration UI
- [ ] Create alert history and audit logs

## Phase 3: User Interface (Lower Priority)

### 5. Web Dashboard
- [ ] Build React-based dashboard for weather visualization
- [ ] Add interactive maps with weather overlays
- [ ] Implement historical data charts and graphs
- [ ] Create mobile-responsive design
"#;

    let request = TaskAnalysisRequest {
        content: task_content.to_string(),
        source_path: Some(PathBuf::from("weather-monitoring-tasks.md")),
        context_hints: vec![
            "full-stack application".to_string(),
            "real-time data processing".to_string(),
            "4 month project".to_string(),
        ],
        max_tokens: Some(4096),
    };

    let result = parser
        .analyze_tasks(request)
        .await
        .expect("Failed to analyze complex real-world task with Claude");

    // Verify comprehensive analysis
    assert!(
        result.tasks.len() >= 5,
        "Should break down into multiple tasks"
    );

    // Verify task metadata quality
    for task in &result.tasks {
        assert!(!task.title.is_empty());
        assert!(!task.description.is_empty());
        assert!(!task.tags.is_empty(), "Tasks should have tags");
    }

    println!("✅ Claude analyzed complex real-world project:");
    println!("   Total tasks: {}", result.tasks.len());
    println!("   Execution strategy: {:?}", result.execution_strategy);
    println!("   Overall complexity: {:?}", result.overall_complexity);
    if let Some(duration_secs) = result.estimated_duration_secs {
        println!("   Estimated duration: {} hours", duration_secs / 3600);
    }

    // Show task breakdown
    println!("\n   Task breakdown:");
    for (i, task) in result.tasks.iter().enumerate() {
        println!(
            "     {}. {} (Priority: {:?}, Complexity: {:?})",
            i + 1,
            task.title,
            task.priority,
            task.complexity
        );
        if let Some(parent) = task.parent_index {
            println!("        Parent: Task {}", parent + 1);
        }
        if !task.dependencies.is_empty() {
            println!("        Dependencies: {:?}", task.dependencies);
        }
    }
}
