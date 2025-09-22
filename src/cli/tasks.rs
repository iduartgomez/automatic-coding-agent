//! Task input parsing and file handling
//!
//! This module handles different task input formats:
//! - Single file tasks (--task-file): Any UTF-8 file becomes a task
//! - Task lists (--tasks): Files containing multiple task specifications
//! - Reference resolution: Tasks can reference other files for context

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
}

#[derive(Debug, Clone)]
pub enum TaskInput {
    SingleFile(PathBuf),      // --task-file (any UTF-8 file)
    TaskList(PathBuf),        // --tasks (any UTF-8 file)
    ConfigWithTasks(PathBuf), // --config (current TOML format)
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
}
