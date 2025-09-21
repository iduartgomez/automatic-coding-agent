//! CLI-specific functionality for the automatic coding agent
//!
//! This module contains all CLI-related code including argument parsing,
//! task input handling, and configuration discovery.

pub mod args;
pub mod config;
pub mod tasks;

pub use args::{Args, ExecutionMode, BatchConfig, InteractiveConfig};
pub use config::{ConfigDiscovery, DefaultAgentConfig};
pub use tasks::{TaskInput, TaskLoader, SimpleTask, FileError};