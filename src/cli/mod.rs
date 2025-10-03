//! # CLI Interface and Task Input Processing
//!
//! Command-line interface for the automatic coding agent, providing both simple
//! task loading and intelligent LLM-based task decomposition.
//!
//! ## Core Components
//!
//! - **[`Args`]**: Command-line argument parsing and validation
//! - **[`TaskLoader`]**: Simple task parsing from files and markdown
//! - **[`IntelligentTaskParser`]**: LLM-powered task decomposition and analysis
//! - **[`ConfigDiscovery`]**: Configuration file discovery and loading
//!
//! ## Key Features
//!
//! ### 🤖 Intelligent Task Parser
//! - **LLM-based decomposition**: Analyzes complex task descriptions and breaks them into structured hierarchies
//! - **Markdown file resolution**: Automatically follows `[text](file.md)` links and includes referenced content
//! - **Detail preservation**: 6 high-level tasks → 42+ detailed subtasks with technical specs
//! - **Dependency mapping**: Automatic TaskId generation and dependency graph construction
//! - **System message support**: Clean instruction separation via `--append-system-prompt`
//!
//! ### 📋 Simple Task Loading
//! - **Multiple input formats**: Files, TOML configs, markdown lists
//! - **Flexible parsing**: Supports various markdown formats and task descriptions
//! - **Fast processing**: Direct parsing without LLM overhead
//!
//! ### ⚙️ Configuration Management
//! - **Auto-discovery**: Finds `.aca.toml` config files in workspace hierarchy
//! - **Default configs**: Sensible defaults for quick start
//! - **Environment integration**: Supports environment variable overrides
//!
//! ## Task Input Modes
//!
//! ### Simple Mode (TaskLoader)
//! Use for straightforward task lists that don't need decomposition:
//! ```markdown
//! # My Tasks
//! - [ ] Task 1: Do something
//! - [ ] Task 2: Do something else
//! ```
//!
//! ### Intelligent Mode (IntelligentTaskParser)
//! Use for complex tasks requiring analysis and breakdown:
//! ```markdown
//! ## Database Migration System
//! → Details: [migration-spec.md](migration-spec.md)
//!
//! Requirements:
//! - PostgreSQL 14+ support
//! - Zero-downtime migrations
//! ```
//!
//! The parser reads linked files and creates detailed execution plans.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use aca::cli::{TaskLoader, IntelligentTaskParser, TaskInput};
//! use aca::llm::{ProviderConfig, ClaudeProvider};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Simple task loading
//!     let input = TaskInput::File(PathBuf::from("tasks.md"));
//!     let plan = TaskLoader::load_and_convert(input).await?;
//!
//!     // Intelligent parsing with LLM
//!     let config = ProviderConfig::default(); // Uses Claude CLI by default
//!     let provider = ClaudeProvider::new(config, PathBuf::from(".")).await?;
//!     let parser = IntelligentTaskParser::new(Box::new(provider));
//!     let plan = parser.parse_file(PathBuf::from("complex-tasks.md")).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod args;
pub mod config;
pub mod intelligent_parser;
pub mod tasks;

pub use args::{Args, BatchConfig, ExecutionMode, InteractiveConfig};
pub use config::{ConfigDiscovery, DefaultAgentConfig};
pub use intelligent_parser::{
    AnalyzedTask, ExecutionStrategy, IntelligentParserError, IntelligentTaskParser,
    TaskAnalysisRequest, TaskAnalysisResult,
};
pub use tasks::{FileError, SimpleTask, TaskInput, TaskLoader};
