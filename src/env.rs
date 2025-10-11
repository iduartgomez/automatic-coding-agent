//! Environment constants and path utilities for the automatic coding agent.
//!
//! This module centralizes all hardcoded paths and directory names used throughout
//! the application, making them easier to maintain and modify.

/// Main application directory name (hidden directory like .git, .vscode)
pub const ACA_DIR_NAME: &str = ".aca";

/// Configuration file name
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// Session-related directory and file names
pub mod session {
    /// Sessions directory name within .aca
    pub const SESSIONS_DIR_NAME: &str = "sessions";

    /// Session metadata directory name
    pub const META_DIR_NAME: &str = "meta";

    /// Session state file name
    pub const SESSION_FILE_NAME: &str = "session.json";

    /// Checkpoints directory name
    pub const CHECKPOINTS_DIR_NAME: &str = "checkpoints";

    /// Logs directory name
    pub const LOGS_DIR_NAME: &str = "logs";

    /// Claude interactions subdirectory name
    pub const CLAUDE_INTERACTIONS_DIR_NAME: &str = "claude_interactions";

    /// OpenAI interactions subdirectory name
    pub const OPENAI_INTERACTIONS_DIR_NAME: &str = "openai_interactions";

    /// Errors subdirectory name
    pub const ERRORS_DIR_NAME: &str = "errors";

    /// Temp directory name
    pub const TEMP_DIR_NAME: &str = "temp";
}

/// Test-related constants
pub mod test {
    /// Default test directory path for temporary operations
    pub const DEFAULT_TEST_DIR: &str = "/tmp";

    /// Test session identifier
    pub const TEST_SESSION_ID: &str = "test-session";
}

/// Common path utilities
use std::path::PathBuf;

/// Build the main .aca directory path from a workspace root
pub fn aca_dir_path(workspace_root: &std::path::Path) -> PathBuf {
    workspace_root.join(ACA_DIR_NAME)
}

/// Build the sessions directory path from a workspace root
pub fn sessions_dir_path(workspace_root: &std::path::Path) -> PathBuf {
    aca_dir_path(workspace_root).join(session::SESSIONS_DIR_NAME)
}

/// Build a specific session directory path
pub fn session_dir_path(workspace_root: &std::path::Path, session_id: &str) -> PathBuf {
    sessions_dir_path(workspace_root).join(session_id)
}

/// Build the session metadata directory path
pub fn session_meta_dir_path(workspace_root: &std::path::Path, session_id: &str) -> PathBuf {
    session_dir_path(workspace_root, session_id).join(session::META_DIR_NAME)
}

/// Build the session checkpoints directory path
pub fn session_checkpoints_dir_path(workspace_root: &std::path::Path, session_id: &str) -> PathBuf {
    session_dir_path(workspace_root, session_id).join(session::CHECKPOINTS_DIR_NAME)
}

/// Build the session logs directory path
pub fn session_logs_dir_path(workspace_root: &std::path::Path, session_id: &str) -> PathBuf {
    session_dir_path(workspace_root, session_id).join(session::LOGS_DIR_NAME)
}

/// Build the Claude interactions logs directory path
pub fn claude_interactions_dir_path(workspace_root: &std::path::Path, session_id: &str) -> PathBuf {
    session_logs_dir_path(workspace_root, session_id).join(session::CLAUDE_INTERACTIONS_DIR_NAME)
}

/// Build the OpenAI interactions logs directory path
pub fn openai_interactions_dir_path(workspace_root: &std::path::Path, session_id: &str) -> PathBuf {
    session_logs_dir_path(workspace_root, session_id).join(session::OPENAI_INTERACTIONS_DIR_NAME)
}

/// Build the session state file path
pub fn session_state_file_path(workspace_root: &std::path::Path, session_id: &str) -> PathBuf {
    session_meta_dir_path(workspace_root, session_id).join(session::SESSION_FILE_NAME)
}

/// Build a checkpoint file path
pub fn checkpoint_file_path(
    workspace_root: &std::path::Path,
    session_id: &str,
    checkpoint_id: &str,
) -> PathBuf {
    session_checkpoints_dir_path(workspace_root, session_id).join(format!("{}.json", checkpoint_id))
}

/// Build config directory path in user's home directory
pub fn user_config_dir_path(home_dir: &std::path::Path) -> PathBuf {
    home_dir.join(ACA_DIR_NAME)
}

/// Build config file path in user's home directory
pub fn user_config_file_path(home_dir: &std::path::Path) -> PathBuf {
    user_config_dir_path(home_dir).join(CONFIG_FILE_NAME)
}

/// Build local config file path in current directory
pub fn local_config_file_path(current_dir: &std::path::Path) -> PathBuf {
    current_dir.join(ACA_DIR_NAME).join(CONFIG_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_path_construction() {
        let workspace = Path::new("/test/workspace");
        let session_id = "test-session-123";

        assert_eq!(aca_dir_path(workspace), Path::new("/test/workspace/.aca"));

        assert_eq!(
            sessions_dir_path(workspace),
            Path::new("/test/workspace/.aca/sessions")
        );

        assert_eq!(
            session_dir_path(workspace, session_id),
            Path::new("/test/workspace/.aca/sessions/test-session-123")
        );

        assert_eq!(
            session_state_file_path(workspace, session_id),
            Path::new("/test/workspace/.aca/sessions/test-session-123/meta/session.json")
        );

        assert_eq!(
            openai_interactions_dir_path(workspace, session_id),
            Path::new("/test/workspace/.aca/sessions/test-session-123/logs/openai_interactions")
        );

        assert_eq!(
            checkpoint_file_path(workspace, session_id, "checkpoint-456"),
            Path::new(
                "/test/workspace/.aca/sessions/test-session-123/checkpoints/checkpoint-456.json"
            )
        );
    }

    #[test]
    fn test_config_paths() {
        let home_dir = Path::new("/home/user");
        let current_dir = Path::new("/current/project");

        assert_eq!(
            user_config_file_path(home_dir),
            Path::new("/home/user/.aca/config.toml")
        );

        assert_eq!(
            local_config_file_path(current_dir),
            Path::new("/current/project/.aca/config.toml")
        );
    }
}
