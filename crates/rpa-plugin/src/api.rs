// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Plugin API traits and types
//!
//! This module defines the interface that plugins must implement.

use crate::permissions::PermissionSet;
use crate::Result;
use async_trait::async_trait;
use rpa_core::{action::ActionResult, Event};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current plugin API version
pub const API_VERSION: &str = "0.1.0";

/// Metadata about a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique identifier for the plugin
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Plugin description
    pub description: Option<String>,
    /// Author information
    pub author: Option<String>,
    /// License (SPDX identifier)
    pub license: Option<String>,
    /// Plugin API version this plugin was built for
    pub api_version: String,
    /// Permissions this plugin requires
    pub required_permissions: PermissionSet,
    /// Custom metadata
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl PluginMetadata {
    /// Create new plugin metadata
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: None,
            author: None,
            license: None,
            api_version: API_VERSION.to_string(),
            required_permissions: PermissionSet::empty(),
            extra: HashMap::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set license
    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    /// Set required permissions
    pub fn with_permissions(mut self, perms: PermissionSet) -> Self {
        self.required_permissions = perms;
        self
    }
}

/// Context passed to plugin during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    /// The event that triggered this plugin
    pub event: Event,
    /// Configuration provided to the plugin
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    /// Working directory for the plugin
    pub work_dir: Option<String>,
    /// Environment variables available to the plugin
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl PluginContext {
    /// Create a new plugin context
    pub fn new(event: Event) -> Self {
        Self {
            event,
            config: HashMap::new(),
            work_dir: None,
            env: HashMap::new(),
        }
    }

    /// Add configuration value
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    /// Set working directory
    pub fn with_work_dir(mut self, dir: impl Into<String>) -> Self {
        self.work_dir = Some(dir.into());
        self
    }
}

/// Result of a plugin action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Output data from the plugin
    #[serde(default)]
    pub output: serde_json::Value,
    /// Logs produced during execution
    #[serde(default)]
    pub logs: Vec<PluginLog>,
}

impl PluginActionResult {
    /// Create a successful result
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            output: serde_json::Value::Null,
            logs: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            output: serde_json::Value::Null,
            logs: Vec::new(),
        }
    }

    /// Add output data
    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = output;
        self
    }

    /// Convert to core ActionResult
    pub fn into_action_result(self) -> ActionResult {
        ActionResult {
            success: self.success,
            message: self.message,
            output: self.output,
            affected_paths: Vec::new(),
        }
    }
}

/// Log entry from a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginLog {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Log levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Trait for plugin actions that can be executed
#[async_trait]
pub trait PluginAction: Send + Sync {
    /// Execute the action with the given context
    async fn execute(&self, ctx: &PluginContext) -> Result<PluginActionResult>;

    /// Get the name of this action
    fn name(&self) -> &str;

    /// Validate the action configuration
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Main plugin trait
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    /// Get available actions provided by this plugin
    fn actions(&self) -> Vec<String>;

    /// Execute an action by name
    async fn execute_action(
        &self,
        action: &str,
        ctx: &PluginContext,
    ) -> Result<PluginActionResult>;
}

/// Host functions that plugins can call (through the sandbox)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostRequest {
    /// Read a file (requires ReadPath permission)
    ReadFile { path: String },
    /// Write a file (requires WritePath permission)
    WriteFile { path: String, content: Vec<u8> },
    /// List directory contents (requires ReadPath permission)
    ListDir { path: String },
    /// Get environment variable (requires Env permission)
    GetEnv { name: String },
    /// Log a message
    Log { level: LogLevel, message: String },
    /// Get current time
    CurrentTime,
    /// Generate UUID
    GenerateUuid,
}

/// Response from host to plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostResponse {
    /// Success with optional data
    Success { data: Option<serde_json::Value> },
    /// Error response
    Error { message: String },
    /// Permission denied
    PermissionDenied { permission: String },
}

impl HostResponse {
    /// Create a success response
    pub fn success() -> Self {
        Self::Success { data: None }
    }

    /// Create a success response with data
    pub fn success_with_data(data: serde_json::Value) -> Self {
        Self::Success { data: Some(data) }
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    /// Create a permission denied response
    pub fn permission_denied(permission: impl Into<String>) -> Self {
        Self::PermissionDenied {
            permission: permission.into(),
        }
    }
}
