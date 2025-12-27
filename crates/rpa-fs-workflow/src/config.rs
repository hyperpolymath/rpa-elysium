// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Workflow configuration parsing
//!
//! Supports both JSON configuration and Nickel configuration files.
//! Nickel files are evaluated and converted to JSON for parsing.

use crate::actions::ActionConfig;
use rpa_core::{Error, Result, Workflow};
use rpa_plugin::{Permission, PermissionSet, SandboxConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Complete workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Basic workflow metadata
    #[serde(flatten)]
    pub workflow: Workflow,

    /// Directories to watch
    pub watch: Vec<WatchConfig>,

    /// Rules that match events to actions
    pub rules: Vec<RuleConfig>,

    /// Plugin configurations
    #[serde(default)]
    pub plugins: Vec<PluginLoadConfig>,
}

/// Configuration for loading a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginLoadConfig {
    /// Path to the plugin WASM file
    pub path: PathBuf,
    /// Optional plugin ID (defaults to filename)
    pub id: Option<String>,
    /// Whether plugin is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Sandbox configuration
    #[serde(default)]
    pub sandbox: PluginSandboxConfig,
}

/// Sandbox configuration for plugins
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginSandboxConfig {
    /// Memory limit in bytes (default: 64MB)
    #[serde(default = "default_memory_limit")]
    pub memory_limit: u64,
    /// Timeout in milliseconds (default: 30s)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Paths the plugin can read
    #[serde(default)]
    pub read_paths: Vec<PathBuf>,
    /// Paths the plugin can write
    #[serde(default)]
    pub write_paths: Vec<PathBuf>,
    /// Environment variables the plugin can access
    #[serde(default)]
    pub env_vars: Vec<String>,
}

fn default_memory_limit() -> u64 {
    64 * 1024 * 1024 // 64MB
}

fn default_timeout() -> u64 {
    30_000 // 30 seconds
}

impl PluginSandboxConfig {
    /// Convert to rpa_plugin::SandboxConfig
    pub fn to_sandbox_config(&self) -> SandboxConfig {
        let mut permissions = PermissionSet::empty()
            .with(Permission::Time)
            .with(Permission::Random);

        for path in &self.read_paths {
            permissions.add(Permission::read_path(path.clone()));
        }
        for path in &self.write_paths {
            permissions.add(Permission::write_path(path.clone()));
        }
        for var in &self.env_vars {
            permissions.add(Permission::env(var.clone()));
        }

        SandboxConfig {
            memory_limit: self.memory_limit,
            timeout_ms: self.timeout_ms,
            fuel_limit: Some(100_000_000),
            permissions,
            work_dir: None,
        }
    }
}

/// Configuration for a watched directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Path to watch
    pub path: PathBuf,
    /// Whether to watch recursively
    #[serde(default = "default_recursive")]
    pub recursive: bool,
}

fn default_recursive() -> bool {
    true
}

/// A rule that matches events and triggers actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// Name of this rule
    pub name: String,
    /// File patterns to match (glob patterns)
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Event types to match
    #[serde(default = "default_events")]
    pub events: Vec<EventType>,
    /// Actions to execute when rule matches
    pub actions: Vec<ActionConfig>,
    /// Whether this rule is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_events() -> Vec<EventType> {
    vec![EventType::Created, EventType::Modified]
}

fn default_enabled() -> bool {
    true
}

/// Event types that can be matched
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

impl WorkflowConfig {
    /// Load configuration from a file (JSON or Nickel)
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "json" => Self::load_json(path),
            "ncl" => Self::load_nickel(path),
            _ => Err(Error::Config(format!(
                "Unsupported config format: {}. Use .json or .ncl",
                extension
            ))),
        }
    }

    /// Load from JSON file
    fn load_json(path: &Path) -> Result<Self> {
        debug!("Loading JSON config from {}", path.display());
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load from Nickel file
    fn load_nickel(path: &Path) -> Result<Self> {
        debug!("Loading Nickel config from {}", path.display());

        // For initial implementation, we use nickel CLI to export to JSON
        // Future: Use nickel-lang-core directly for better integration
        let output = std::process::Command::new("nickel")
            .args(["export", "--format", "json"])
            .arg(path)
            .output()
            .map_err(|e| Error::Config(format!(
                "Failed to run nickel: {}. Ensure nickel is installed.",
                e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Config(format!("Nickel evaluation failed: {}", stderr)));
        }

        let json = String::from_utf8_lossy(&output.stdout);
        let config: Self = serde_json::from_str(&json)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.watch.is_empty() {
            return Err(Error::Config("At least one watch path is required".into()));
        }

        if self.rules.is_empty() {
            return Err(Error::Config("At least one rule is required".into()));
        }

        for (i, rule) in self.rules.iter().enumerate() {
            if rule.name.is_empty() {
                return Err(Error::Config(format!("Rule {} has no name", i)));
            }
            if rule.actions.is_empty() {
                return Err(Error::Config(format!(
                    "Rule '{}' has no actions",
                    rule.name
                )));
            }
        }

        Ok(())
    }

    /// Create a minimal example configuration
    pub fn example() -> Self {
        Self {
            workflow: Workflow::new("example-workflow")
                .with_description("Example filesystem workflow"),
            watch: vec![WatchConfig {
                path: PathBuf::from("/tmp/watch"),
                recursive: true,
            }],
            rules: vec![RuleConfig {
                name: "backup-pdfs".to_string(),
                patterns: vec!["*.pdf".to_string()],
                events: vec![EventType::Created],
                actions: vec![ActionConfig::Copy {
                    destination: PathBuf::from("/tmp/backup"),
                    overwrite: false,
                    preserve_structure: false,
                }],
                enabled: true,
            }],
            plugins: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_config() {
        let config = WorkflowConfig::example();
        assert!(config.validate().is_ok());
        assert_eq!(config.workflow.name, "example-workflow");
    }

    #[test]
    fn test_json_roundtrip() {
        let config = WorkflowConfig::example();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: WorkflowConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.workflow.name, config.workflow.name);
    }
}
