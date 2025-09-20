//! # Session Management System
//!
//! Provides comprehensive session lifecycle management with atomic persistence,
//! checkpoint creation, and intelligent recovery capabilities for long-running
//! coding automation sessions.
//!
//! ## Core Components
//!
//! - **[`SessionManager`]**: Central orchestrator for session lifecycle
//! - **[`PersistenceManager`]**: Atomic file operations with transaction support
//! - **[`RecoveryManager`]**: State validation and corruption recovery
//! - **[`SessionMetadata`]**: Version tracking and performance metrics
//!
//! ## Key Features
//!
//! ### 💾 Atomic Persistence
//! - Thread-safe file operations with rollback capability
//! - Transactional writes with integrity validation
//! - Automatic backup creation before modifications
//!
//! ### 🔄 Checkpoint System
//! - UUID-based checkpoint creation and management
//! - Automatic cleanup of old checkpoints
//! - Fast restoration from any checkpoint
//!
//! ### 🛡️ Recovery & Validation
//! - Comprehensive state integrity checking
//! - Automatic corruption detection and repair
//! - Graceful degradation and error recovery
//!
//! ### 📊 Performance Tracking
//! - Session duration and activity metrics
//! - Memory usage and resource consumption
//! - Task completion statistics
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use automatic_coding_agent::session::{SessionManager, SessionManagerConfig, SessionInitOptions};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = SessionManagerConfig::default();
//!     let init_options = SessionInitOptions {
//!         name: "My Coding Session".to_string(),
//!         workspace_root: std::env::current_dir()?,
//!         enable_auto_save: true,
//!         ..Default::default()
//!     };
//!
//!     let session_manager = SessionManager::new(
//!         PathBuf::from("/path/to/session"),
//!         config,
//!         init_options
//!     ).await?;
//!
//!     // Session state is automatically persisted
//!     Ok(())
//! }
//! ```

/// Central session lifecycle management and coordination.
///
/// The [`SessionManager`] orchestrates all session operations including
/// persistence, recovery, and state management.
pub mod manager;

/// Session metadata and configuration tracking.
///
/// Handles version information, performance metrics, and session
/// configuration state.
pub mod metadata;

/// Atomic persistence operations and transaction support.
///
/// Provides thread-safe file operations with rollback capabilities
/// and integrity validation for session state.
pub mod persistence;

/// State validation and corruption recovery.
///
/// Handles integrity checking, automatic recovery from corruption,
/// and graceful error handling for session data.
pub mod recovery;

#[cfg(test)]
mod tests;

pub use manager::*;
pub use metadata::*;
pub use persistence::*;
pub use recovery::*;
