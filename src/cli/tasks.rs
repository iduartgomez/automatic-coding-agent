//! Task input parsing and file handling
//!
//! This module handles different task input formats:
//! - Single file tasks (--task-file): Any UTF-8 file becomes a task
//! - Task lists (--tasks): Files containing multiple task specifications
//! - Reference resolution: Tasks can reference other files for context

use crate::task::{
    ComplexityLevel, ContextRequirements, ExecutionPlan, FileImportance, FileRef, TaskMetadata,
    TaskPriority, TaskSpec,
};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Debug, Error)]
pub enum FileError {
    #[error("File '{path}' is not UTF-8 encoded: {hint}")]
    NotUtf8 { path: PathBuf, hint: String },

    #[error("File '{path}' not found")]
    NotFound { path: PathBuf },

    #[error("IO error reading '{path}': {source}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Referenced file '{path}' could not be loaded: {reason}")]
    ReferenceError { path: PathBuf, reason: String },

    #[error("Task parsing error in '{path}': {reason}")]
    ParseError { path: PathBuf, reason: String },

    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone)]
pub enum TaskInput {
    SingleFile(PathBuf),      // --task-file (any UTF-8 file)
    TaskList(PathBuf),        // --tasks (any UTF-8 file)
    ConfigWithTasks(PathBuf), // --config (current TOML format)
    ExecutionPlan(PathBuf),   // --execution-plan (JSON or TOML execution plan)
}

#[derive(Debug)]
struct Utf8File {
    path: PathBuf,
    content: String,
}

#[derive(Debug, Clone)]
pub struct SimpleTask {
    pub description: String,
    pub reference_file: Option<PathBuf>,
}

/// Task loader responsible for loading and parsing different task input formats
pub struct TaskLoader;

impl TaskLoader {
    /// Convert a TaskInput to an ExecutionPlan using intelligent or naive parser
    pub async fn task_input_to_execution_plan_with_options(
        input: &TaskInput,
        use_intelligent: bool,
        context_hints: Vec<String>,
    ) -> Result<ExecutionPlan, FileError> {
        match input {
            TaskInput::ExecutionPlan(path) => Self::load_execution_plan(path),
            _ => {
                if use_intelligent {
                    Self::task_input_to_execution_plan_intelligent(input, context_hints).await
                } else {
                    Self::task_input_to_execution_plan(input)
                }
            }
        }
    }

    /// Load an execution plan from a JSON or TOML file
    pub fn load_execution_plan(path: &std::path::Path) -> Result<ExecutionPlan, FileError> {
        let content = std::fs::read_to_string(path).map_err(|e| FileError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("json");

        let plan: ExecutionPlan = match extension {
            "json" => serde_json::from_str(&content).map_err(|e| FileError::Parse(format!(
                "Failed to parse JSON execution plan: {}",
                e
            )))?,
            "toml" => toml::from_str(&content).map_err(|e| FileError::Parse(format!(
                "Failed to parse TOML execution plan: {}",
                e
            )))?,
            _ => {
                return Err(FileError::Parse(format!(
                    "Unsupported execution plan format: {}. Use .json or .toml",
                    extension
                )))
            }
        };

        Ok(plan)
    }

    /// Convert a TaskInput to an ExecutionPlan using intelligent parser
    async fn task_input_to_execution_plan_intelligent(
        input: &TaskInput,
        context_hints: Vec<String>,
    ) -> Result<ExecutionPlan, FileError> {
        use crate::cli::IntelligentTaskParser;
        use crate::llm::provider::LLMProviderFactory;
        use crate::llm::types::{ProviderConfig, ProviderType};

        // Create LLM provider
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            FileError::Parse(
                "ANTHROPIC_API_KEY environment variable required for intelligent parser".to_string(),
            )
        })?;

        let provider_config = ProviderConfig {
            provider_type: ProviderType::Claude,
            api_key: Some(api_key),
            base_url: None,
            model: Some("claude-sonnet".to_string()),
            rate_limits: Default::default(),
            additional_config: Default::default(),
        };

        let workspace = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let provider = LLMProviderFactory::create_provider(provider_config, workspace)
            .await
            .map_err(|e| FileError::Parse(format!("Failed to create LLM provider: {}", e)))?;

        let parser = IntelligentTaskParser::new(provider);

        // Parse based on input type
        match input {
            TaskInput::SingleFile(path) | TaskInput::TaskList(path) => {
                parser
                    .parse_file(path.clone(), context_hints)
                    .await
                    .map_err(|e| FileError::Parse(format!("Intelligent parsing failed: {}", e)))
            }
            TaskInput::ConfigWithTasks(_) => {
                // For TOML configs, fall back to naive parsing
                Self::task_input_to_execution_plan(input)
            }
            TaskInput::ExecutionPlan(_) => {
                unreachable!("ExecutionPlan should be handled by task_input_to_execution_plan_with_options")
            }
        }
    }

    /// Load a UTF-8 file with proper error handling
    fn load_utf8_file<P: AsRef<Path>>(path: P) -> Result<Utf8File, FileError> {
        let path = path.as_ref().to_path_buf();

        debug!("Loading UTF-8 file: {:?}", path);

        match fs::read_to_string(&path) {
            Ok(content) => {
                debug!(
                    "Successfully loaded {} characters from {:?}",
                    content.len(),
                    path
                );
                Ok(Utf8File { path, content })
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => Err(FileError::NotFound { path }),
                std::io::ErrorKind::InvalidData => Err(FileError::NotUtf8 {
                    path,
                    hint:
                        "File appears to be binary. Only UTF-8 text files are supported for tasks."
                            .to_string(),
                }),
                _ => Err(FileError::IoError { path, source: e }),
            },
        }
    }

    /// Parse a single file as a task (--task-file)
    pub fn parse_single_file_task<P: AsRef<Path>>(path: P) -> Result<SimpleTask, FileError> {
        let file = Self::load_utf8_file(path)?;

        debug!("Parsing single file task from: {:?}", file.path);

        Ok(SimpleTask {
            description: file.content,
            reference_file: None,
        })
    }

    /// Parse a task list file (--tasks)
    pub fn parse_task_list<P: AsRef<Path>>(path: P) -> Result<Vec<SimpleTask>, FileError> {
        let file = Self::load_utf8_file(path)?;

        debug!("Parsing task list from: {:?}", file.path);

        let tasks = Self::parse_task_list_content(&file.content, &file.path)?;

        debug!("Parsed {} tasks from {:?}", tasks.len(), file.path);

        Ok(tasks)
    }

    /// Parse task list content - handles various text formats
    fn parse_task_list_content(
        content: &str,
        source_path: &Path,
    ) -> Result<Vec<SimpleTask>, FileError> {
        let mut tasks = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }

            // Handle various task list formats
            let task = if let Some(task) = Self::parse_task_line(line, source_path)? {
                task
            } else {
                continue; // Skip unrecognized lines
            };

            debug!(
                "Parsed task from line {}: {}",
                line_num + 1,
                &task.description[..task.description.len().min(50)]
            );
            tasks.push(task);
        }

        if tasks.is_empty() {
            warn!("No tasks found in file: {:?}", source_path);
        }

        Ok(tasks)
    }

    /// Parse a single task line with optional reference
    fn parse_task_line(line: &str, source_path: &Path) -> Result<Option<SimpleTask>, FileError> {
        // Handle reference syntax: "task description -> reference_file.md"
        if let Some((description, reference)) = line.split_once(" -> ") {
            let description = description.trim().to_string();
            let reference_path = Self::resolve_reference_path(reference.trim(), source_path)?;

            Ok(Some(SimpleTask {
                description,
                reference_file: Some(reference_path),
            }))
        } else {
            // Handle various task list formats
            let description = Self::extract_task_description(line);

            if description.is_empty() {
                Ok(None)
            } else {
                Ok(Some(SimpleTask {
                    description,
                    reference_file: None,
                }))
            }
        }
    }

    /// Extract task description from various formats (markdown, org-mode, etc.)
    fn extract_task_description(line: &str) -> String {
        let line = line.trim();

        // Markdown task list: "- [ ] task" or "- [x] task" or "* task"
        if let Some(rest) = line
            .strip_prefix("- [ ]")
            .or_else(|| line.strip_prefix("- [x]"))
        {
            return rest.trim().to_string();
        }
        if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            return rest.trim().to_string();
        }

        // Org-mode tasks: "* TODO task" or "* DONE task"
        if let Some(rest) = line
            .strip_prefix("* TODO ")
            .or_else(|| line.strip_prefix("* DONE "))
        {
            return rest.trim().to_string();
        }
        if let Some(rest) = line.strip_prefix("* ") {
            return rest.trim().to_string();
        }

        // Numbered list: "1. task" or "1) task"
        if let Some(pos) = line.find(". ").or_else(|| line.find(") "))
            && line[..pos].chars().all(|c| c.is_ascii_digit())
        {
            return line[pos + 2..].trim().to_string();
        }

        // Plain text - assume the whole line is a task
        line.to_string()
    }

    /// Resolve reference file path relative to the source file
    fn resolve_reference_path(reference: &str, source_path: &Path) -> Result<PathBuf, FileError> {
        let reference_path = if Path::new(reference).is_absolute() {
            PathBuf::from(reference)
        } else {
            // Resolve relative to the source file's directory
            if let Some(parent) = source_path.parent() {
                parent.join(reference)
            } else {
                PathBuf::from(reference)
            }
        };

        debug!(
            "Resolved reference '{}' to: {:?}",
            reference, reference_path
        );

        Ok(reference_path)
    }

    /// Load and resolve all references for a list of tasks
    pub fn resolve_task_references(tasks: &mut [SimpleTask]) -> Result<(), FileError> {
        for task in tasks {
            if let Some(ref_path) = &task.reference_file {
                match Self::load_utf8_file(ref_path) {
                    Ok(ref_file) => {
                        debug!("Loaded reference file: {:?}", ref_path);
                        // Append reference content to task description
                        task.description.push_str("\n\n--- Reference from ");
                        task.description.push_str(&ref_path.to_string_lossy());
                        task.description.push_str(" ---\n");
                        task.description.push_str(&ref_file.content);
                    }
                    Err(e) => {
                        let reason = format!("{}", e);
                        return Err(FileError::ReferenceError {
                            path: ref_path.clone(),
                            reason,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Convert a single SimpleTask to a TaskSpec
    fn simple_task_to_task_spec(simple_task: SimpleTask) -> TaskSpec {
        let mut context_requirements = ContextRequirements::default();

        // If the task has a reference file, add it to the context requirements
        if let Some(ref_path) = &simple_task.reference_file {
            context_requirements.required_files.push(ref_path.clone());
        }

        TaskSpec {
            title: format!(
                "Task: {}",
                if simple_task.description.len() > 50 {
                    format!("{}...", &simple_task.description[..47])
                } else {
                    simple_task.description.clone()
                }
            ),
            description: simple_task.description,
            dependencies: Vec::new(),
            metadata: TaskMetadata {
                priority: TaskPriority::Normal,
                estimated_complexity: Some(ComplexityLevel::Moderate),
                estimated_duration: Some(
                    chrono::Duration::from_std(std::time::Duration::from_secs(300)).unwrap(),
                ),
                repository_refs: Vec::new(),
                file_refs: simple_task
                    .reference_file
                    .map(|p| {
                        vec![FileRef {
                            path: p,
                            repository: "local".to_string(),
                            line_range: None,
                            importance: FileImportance::Medium,
                        }]
                    })
                    .unwrap_or_default(),
                tags: vec!["from-task-file".to_string()],
                context_requirements,
            },
        }
    }

    /// Convert a single file task to an ExecutionPlan
    pub fn single_file_to_execution_plan<P: AsRef<Path>>(
        path: P,
    ) -> Result<ExecutionPlan, FileError> {
        let simple_task = Self::parse_single_file_task(path.as_ref())?;
        let task_spec = Self::simple_task_to_task_spec(simple_task);

        let plan_name = format!(
            "Single File Task: {}",
            path.as_ref()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        );

        let plan = ExecutionPlan::new()
            .with_task(task_spec)
            .with_metadata(plan_name, "Execution plan for single file task")
            .with_tags(vec![
                "single-file".to_string(),
                "auto-generated".to_string(),
            ])
            .with_sequential_execution();

        debug!(
            "Created execution plan for single file: {:?}",
            path.as_ref()
        );
        Ok(plan)
    }

    /// Convert a task list file to an ExecutionPlan
    pub fn task_list_to_execution_plan<P: AsRef<Path>>(
        path: P,
    ) -> Result<ExecutionPlan, FileError> {
        let mut simple_tasks = Self::parse_task_list(path.as_ref())?;

        // Resolve references
        debug!("Resolving task references for execution plan...");
        Self::resolve_task_references(&mut simple_tasks)?;

        // Convert to TaskSpec instances
        let task_specs: Vec<TaskSpec> = simple_tasks
            .into_iter()
            .map(Self::simple_task_to_task_spec)
            .collect();

        let plan_name = format!(
            "Task List: {}",
            path.as_ref()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        );

        let task_count = task_specs.len();
        let estimated_duration =
            chrono::Duration::from_std(std::time::Duration::from_secs(300 * task_count as u64))
                .unwrap_or_else(|_| chrono::Duration::minutes(5));

        let plan = ExecutionPlan::new()
            .with_tasks(task_specs)
            .with_metadata(
                plan_name,
                format!("Execution plan for {} tasks from task list", task_count),
            )
            .with_tags(vec!["task-list".to_string(), "auto-generated".to_string()])
            .with_estimated_duration(estimated_duration)
            .with_sequential_execution();

        debug!(
            "Created execution plan for task list: {:?} with {} tasks",
            path.as_ref(),
            plan.task_count()
        );
        Ok(plan)
    }

    /// Convert TaskInput to ExecutionPlan
    pub fn task_input_to_execution_plan(
        task_input: &TaskInput,
    ) -> Result<ExecutionPlan, FileError> {
        match task_input {
            TaskInput::SingleFile(path) => Self::single_file_to_execution_plan(path),
            TaskInput::TaskList(path) => Self::task_list_to_execution_plan(path),
            TaskInput::ConfigWithTasks(_path) => {
                // This should be handled by the structured config mode, not TaskLoader
                Err(FileError::ParseError {
                    path: _path.clone(),
                    reason: "ConfigWithTasks should be handled by structured config mode"
                        .to_string(),
                })
            }
            TaskInput::ExecutionPlan(path) => Self::load_execution_plan(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_utf8_file() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, "Hello, world!").unwrap();

        let result = TaskLoader::load_utf8_file(temp_file.path()).unwrap();
        assert_eq!(result.content, "Hello, world!");
    }

    #[test]
    fn test_parse_single_file_task() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, "Implement user authentication").unwrap();

        let task = TaskLoader::parse_single_file_task(temp_file.path()).unwrap();
        assert_eq!(task.description, "Implement user authentication");
        assert!(task.reference_file.is_none());
    }

    #[test]
    fn test_parse_task_list_markdown() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(
            &temp_file,
            "- [ ] Fix authentication bug\n\
             - [x] Add tests\n\
             * Update documentation\n\
             # This is a comment\n\
             \n\
             - [ ] Deploy to staging",
        )
        .unwrap();

        let tasks = TaskLoader::parse_task_list(temp_file.path()).unwrap();
        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0].description, "Fix authentication bug");
        assert_eq!(tasks[1].description, "Add tests");
        assert_eq!(tasks[2].description, "Update documentation");
        assert_eq!(tasks[3].description, "Deploy to staging");
    }

    #[test]
    fn test_parse_task_with_reference() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(
            &temp_file,
            "Fix memory leak -> analysis.md\n\
             Add logging",
        )
        .unwrap();

        let tasks = TaskLoader::parse_task_list(temp_file.path()).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].description, "Fix memory leak");
        assert!(tasks[0].reference_file.is_some());
        assert_eq!(tasks[1].description, "Add logging");
        assert!(tasks[1].reference_file.is_none());
    }

    #[test]
    fn test_extract_task_description_formats() {
        assert_eq!(
            TaskLoader::extract_task_description("- [ ] Todo item"),
            "Todo item"
        );
        assert_eq!(
            TaskLoader::extract_task_description("* TODO Org task"),
            "TODO Org task"
        );
        assert_eq!(
            TaskLoader::extract_task_description("1. Numbered task"),
            "Numbered task"
        );
        assert_eq!(
            TaskLoader::extract_task_description("Plain text task"),
            "Plain text task"
        );
    }

    #[test]
    fn test_load_utf8_file_with_binary() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, [0xFF, 0xFE, 0x00, 0x01]).unwrap();

        let result = TaskLoader::load_utf8_file(temp_file.path());
        assert!(result.is_err());

        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("not UTF-8 encoded"));
        assert!(error_msg.contains("binary"));
    }

    #[test]
    fn test_single_file_to_execution_plan() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, "Implement user authentication system").unwrap();

        let plan = TaskLoader::single_file_to_execution_plan(temp_file.path()).unwrap();

        assert_eq!(plan.task_count(), 1);
        assert_eq!(plan.setup_command_count(), 0);
        assert!(plan.has_tasks());
        assert!(!plan.has_setup_commands());

        let task = &plan.task_specs[0];
        assert_eq!(task.description, "Implement user authentication system");
        assert!(task.title.contains("Task:"));
        assert!(task.metadata.tags.contains(&"from-task-file".to_string()));

        assert!(plan.metadata.name.is_some());
        assert!(plan.metadata.tags.contains(&"single-file".to_string()));
    }

    #[test]
    fn test_task_list_to_execution_plan() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(
            &temp_file,
            "- [ ] Fix authentication bug\n\
             - [x] Add comprehensive tests\n\
             * Update documentation\n\
             \n\
             - [ ] Deploy to staging",
        )
        .unwrap();

        let plan = TaskLoader::task_list_to_execution_plan(temp_file.path()).unwrap();

        assert_eq!(plan.task_count(), 4);
        assert_eq!(plan.setup_command_count(), 0);
        assert!(plan.has_tasks());
        assert!(!plan.has_setup_commands());

        let task_descriptions: Vec<&str> = plan
            .task_specs
            .iter()
            .map(|t| t.description.as_str())
            .collect();

        assert!(task_descriptions.contains(&"Fix authentication bug"));
        assert!(task_descriptions.contains(&"Add comprehensive tests"));
        assert!(task_descriptions.contains(&"Update documentation"));
        assert!(task_descriptions.contains(&"Deploy to staging"));

        assert!(plan.metadata.name.is_some());
        assert!(plan.metadata.tags.contains(&"task-list".to_string()));
        assert!(plan.metadata.estimated_duration.is_some());
    }

    #[test]
    fn test_task_input_to_execution_plan() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, "Test task description").unwrap();

        // Test single file input
        let single_input = TaskInput::SingleFile(temp_file.path().to_path_buf());
        let plan = TaskLoader::task_input_to_execution_plan(&single_input).unwrap();
        assert_eq!(plan.task_count(), 1);

        // Test task list input
        let list_input = TaskInput::TaskList(temp_file.path().to_path_buf());
        let plan = TaskLoader::task_input_to_execution_plan(&list_input).unwrap();
        assert_eq!(plan.task_count(), 1);

        // Test config with tasks (should return error)
        let config_input = TaskInput::ConfigWithTasks(temp_file.path().to_path_buf());
        let result = TaskLoader::task_input_to_execution_plan(&config_input);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("structured config mode"));
    }

    #[test]
    fn test_simple_task_to_task_spec() {
        let simple_task = SimpleTask {
            description: "Test task description".to_string(),
            reference_file: Some(PathBuf::from("reference.md")),
        };

        let task_spec = TaskLoader::simple_task_to_task_spec(simple_task);

        assert_eq!(task_spec.description, "Test task description");
        assert!(task_spec.title.starts_with("Task:"));
        assert_eq!(task_spec.metadata.file_refs.len(), 1);
        assert_eq!(
            task_spec.metadata.file_refs[0].path,
            PathBuf::from("reference.md")
        );
        assert_eq!(task_spec.metadata.file_refs[0].repository, "local");
        assert_eq!(
            task_spec.metadata.file_refs[0].importance,
            FileImportance::Medium
        );
        assert!(
            task_spec
                .metadata
                .context_requirements
                .required_files
                .contains(&PathBuf::from("reference.md"))
        );
        assert!(
            task_spec
                .metadata
                .tags
                .contains(&"from-task-file".to_string())
        );
    }
}
