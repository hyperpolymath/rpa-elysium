// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Action traits and types for automation operations

use crate::{Event, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for executable actions
#[async_trait]
pub trait Action: Send + Sync {
    /// Execute the action with the given event context
    async fn execute(&self, event: &Event) -> Result<ActionResult>;

    /// Get the name of this action
    fn name(&self) -> &str;

    /// Validate the action configuration
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Result of an action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Any output data from the action
    #[serde(default)]
    pub output: serde_json::Value,
    /// Paths affected by the action
    #[serde(default)]
    pub affected_paths: Vec<std::path::PathBuf>,
}

impl ActionResult {
    /// Create a successful result
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            output: serde_json::Value::Null,
            affected_paths: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            output: serde_json::Value::Null,
            affected_paths: Vec::new(),
        }
    }

    /// Add affected paths
    pub fn with_paths(mut self, paths: Vec<std::path::PathBuf>) -> Self {
        self.affected_paths = paths;
        self
    }

    /// Add output data
    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = output;
        self
    }
}
