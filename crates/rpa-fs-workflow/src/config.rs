// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Workflow configuration parsing
//!
//! Supports both JSON configuration and Nickel configuration files.
//! Nickel files are evaluated and converted to JSON for parsing.

use crate::actions::ActionConfig;
use rpa_core::{Error, Result, Workflow};
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
