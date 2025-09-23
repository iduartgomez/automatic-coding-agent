//! Configuration discovery and loading
//!
//! This module handles the configuration discovery hierarchy:
//! 1. Current directory: ./aca.toml or ./.aca/config.toml
//! 2. User config: ~/.aca/config.toml
//! 3. System config: /etc/aca/config.toml
//! 4. Built-in defaults

use crate::{
    AgentConfig, claude::ClaudeConfig, env, session::SessionManagerConfig, task::TaskManagerConfig,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::env as std_env;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultAgentConfig {
    pub workspace_path: Option<PathBuf>,
    pub session_config: SessionManagerConfig,
    pub task_config: TaskManagerConfig,
    pub claude_config: ClaudeConfig,
}

impl Default for DefaultAgentConfig {
    fn default() -> Self {
        let default_agent = AgentConfig::default();
        Self {
            workspace_path: None, // Will be set to current dir if not specified
            session_config: default_agent.session_config,
            task_config: default_agent.task_config,
            claude_config: default_agent.claude_config,
        }
    }
}

impl DefaultAgentConfig {
    /// Convert to AgentConfig with specified workspace and empty setup commands
    pub fn to_agent_config(&self, workspace_override: Option<PathBuf>) -> AgentConfig {
        let workspace_path = workspace_override
            .or_else(|| self.workspace_path.clone())
            .unwrap_or_else(|| std_env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        AgentConfig {
            workspace_path,
            setup_commands: Vec::new(), // Will be populated by task processing
            session_config: self.session_config.clone(),
            task_config: self.task_config.clone(),
            claude_config: self.claude_config.clone(),
        }
    }

    /// Load from TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: DefaultAgentConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save to TOML file
    pub fn to_toml_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Configuration discovery system
pub struct ConfigDiscovery;

impl ConfigDiscovery {
    /// Discover and load configuration using the hierarchy
    pub fn discover_config() -> Result<DefaultAgentConfig, Box<dyn std::error::Error>> {
        // Try discovery hierarchy
        if let Some(config_path) = Self::find_config_file() {
            info!("Loading configuration from: {:?}", config_path);
            return DefaultAgentConfig::from_toml_file(config_path);
        }

        info!("No configuration file found, using defaults");
        Ok(DefaultAgentConfig::default())
    }

    /// Find configuration file using discovery hierarchy
    pub fn find_config_file() -> Option<PathBuf> {
        let candidates = Self::get_config_candidates();

        for candidate in candidates {
            debug!("Checking for config file: {:?}", candidate);
            if candidate.exists() && candidate.is_file() {
                debug!("Found config file: {:?}", candidate);
                return Some(candidate);
            }
        }

        debug!("No config file found in discovery hierarchy");
        None
    }

    /// Get list of configuration file candidates in priority order
    fn get_config_candidates() -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        // 1. Current directory: ./aca.toml
        if let Ok(current_dir) = std_env::current_dir() {
            candidates.push(current_dir.join("aca.toml"));
            candidates.push(env::local_config_file_path(&current_dir));
        }

        // 2. User config: ~/.aca/config.toml
        if let Some(home_dir) = Self::get_home_dir() {
            candidates.push(env::user_config_file_path(&home_dir));
        }

        // 3. System config: /etc/aca/config.toml (Unix-like systems)
        #[cfg(unix)]
        candidates.push(PathBuf::from("/etc/aca/config.toml"));

        // Windows system config: C:\ProgramData\aca\config.toml
        #[cfg(windows)]
        if let Ok(program_data) = env::var("PROGRAMDATA") {
            candidates.push(PathBuf::from(program_data).join("aca").join("config.toml"));
        }

        candidates
    }

    /// Get home directory path
    fn get_home_dir() -> Option<PathBuf> {
        std_env::var("HOME")
            .ok()
            .or_else(|| std_env::var("USERPROFILE").ok())
            .map(PathBuf::from)
    }

    /// Create a default config file in the user's home directory
    pub fn create_default_user_config() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = Self::get_home_dir().ok_or("Could not determine home directory")?;

        let config_dir = env::user_config_dir_path(&home_dir);
        let config_path = env::user_config_file_path(&home_dir);

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
            info!("Created configuration directory: {:?}", config_dir);
        }

        // Create default config if it doesn't exist
        if !config_path.exists() {
            let default_config = DefaultAgentConfig::default();
            default_config.to_toml_file(&config_path)?;
            info!("Created default configuration file: {:?}", config_path);
        } else {
            warn!("Configuration file already exists: {:?}", config_path);
        }

        Ok(config_path)
    }

    /// Show configuration discovery information for debugging
    pub fn show_discovery_info() {
        println!("Configuration Discovery Hierarchy:");
        println!();

        let candidates = Self::get_config_candidates();
        for (i, candidate) in candidates.iter().enumerate() {
            let status = if candidate.exists() {
                if candidate.is_file() {
                    "✓ EXISTS"
                } else {
                    "✗ NOT A FILE"
                }
            } else {
                "✗ NOT FOUND"
            };

            println!("  {}. {:?} - {}", i + 1, candidate, status);
        }

        println!();
        if let Some(found) = Self::find_config_file() {
            println!("Active configuration: {:?}", found);
        } else {
            println!("Active configuration: Built-in defaults");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_agent_config() {
        let config = DefaultAgentConfig::default();
        let agent_config = config.to_agent_config(None);

        // Should have current directory as workspace if not specified
        assert!(!agent_config.workspace_path.as_os_str().is_empty());
        assert_eq!(agent_config.setup_commands.len(), 0);
    }

    #[test]
    fn test_config_serialization() {
        let config = DefaultAgentConfig::default();
        let toml_string = toml::to_string(&config).unwrap();

        // Should be able to deserialize back
        let _deserialized: DefaultAgentConfig = toml::from_str(&toml_string).unwrap();
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let original_config = DefaultAgentConfig::default();

        // Save config
        original_config.to_toml_file(&config_path).unwrap();
        assert!(config_path.exists());

        // Load config
        let loaded_config = DefaultAgentConfig::from_toml_file(&config_path).unwrap();

        // Compare key fields (session config should match)
        assert_eq!(
            original_config.session_config.auto_save_interval_minutes,
            loaded_config.session_config.auto_save_interval_minutes
        );
    }

    #[test]
    fn test_config_candidates() {
        let candidates = ConfigDiscovery::get_config_candidates();

        // Should have at least current directory candidates
        assert!(!candidates.is_empty());

        // First candidates should be current directory
        assert!(candidates[0].file_name().unwrap() == "aca.toml");
    }
}
