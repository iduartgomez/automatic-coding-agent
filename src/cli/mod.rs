//! CLI-specific functionality for the automatic coding agent
//!
//! This module contains all CLI-related code including argument parsing,
//! task input handling, and configuration discovery.

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
