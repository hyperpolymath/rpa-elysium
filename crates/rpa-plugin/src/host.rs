// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Plugin host for managing and executing plugins

use crate::api::{PluginContext, PluginMetadata, PluginActionResult};
use crate::error::{PluginError, Result};
use crate::permissions::Permission;
use crate::sandbox::{Sandbox, SandboxConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use wasmtime::Module;

/// Configuration for loading a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Path to the plugin WASM file
    pub path: PathBuf,
    /// Plugin ID (derived from path if not specified)
    pub id: Option<String>,
    /// Whether the plugin is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Sandbox configuration
    #[serde(default)]
    pub sandbox: SandboxConfig,
    /// Plugin-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl PluginConfig {
    /// Create a new plugin configuration
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string());

        Self {
            path,
            id,
            enabled: true,
            sandbox: SandboxConfig::default(),
            config: HashMap::new(),
        }
    }

    /// Set plugin ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a permission
    pub fn with_permission(mut self, perm: Permission) -> Self {
        self.sandbox.permissions.add(perm);
        self
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.sandbox.memory_limit = bytes;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.sandbox.timeout_ms = ms;
        self
    }

    /// Add configuration value
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    /// Get plugin ID
    pub fn get_id(&self) -> String {
        self.id
            .clone()
            .unwrap_or_else(|| {
                self.path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            })
    }
}

/// A loaded plugin instance
pub struct PluginInstance {
    /// Plugin configuration
    config: PluginConfig,
    /// Plugin metadata (loaded from WASM)
    metadata: PluginMetadata,
    /// Compiled WASM module
    module: Module,
    /// Sandbox for execution
    sandbox: Sandbox,
    /// Available actions
    actions: Vec<String>,
}

impl PluginInstance {
    /// Get plugin ID
    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    /// Get plugin metadata
    pub fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    /// Get available actions
    pub fn actions(&self) -> &[String] {
        &self.actions
    }

    /// Check if plugin has an action
    pub fn has_action(&self, action: &str) -> bool {
        self.actions.iter().any(|a| a == action)
    }

    /// Execute an action
    pub fn execute(&self, action: &str, ctx: &PluginContext) -> Result<PluginActionResult> {
        if !self.has_action(action) {
            return Err(PluginError::ExecutionFailed(format!(
                "Plugin '{}' does not have action '{}'",
                self.id(),
                action
            )));
        }

        self.sandbox.execute(&self.module, action, ctx)
    }
}

/// Plugin host that manages plugin lifecycle
pub struct PluginHost {
    /// Loaded plugins by ID
    plugins: HashMap<String, PluginInstance>,
    /// Default sandbox configuration
    default_sandbox_config: SandboxConfig,
    /// Plugin search paths
    search_paths: Vec<PathBuf>,
}

impl PluginHost {
    /// Create a new plugin host
    pub fn new() -> Result<Self> {
        Ok(Self {
            plugins: HashMap::new(),
            default_sandbox_config: SandboxConfig::default(),
            search_paths: Vec::new(),
        })
    }

    /// Add a search path for plugins
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Set default sandbox configuration
    pub fn set_default_sandbox_config(&mut self, config: SandboxConfig) {
        self.default_sandbox_config = config;
    }

    /// Load a plugin from configuration
    pub fn load_plugin(&mut self, config: PluginConfig) -> Result<String> {
        if !config.enabled {
            return Err(PluginError::LoadFailed("Plugin is disabled".to_string()));
        }

        let plugin_id = config.get_id();
        info!("Loading plugin: {} from {}", plugin_id, config.path.display());

        // Create sandbox
        let sandbox = Sandbox::new(config.sandbox.clone())?;

        // Load WASM module
        let module = sandbox.load_module_from_file(&config.path)?;

        // Extract metadata from module (in a real impl, this would parse custom sections)
        let metadata = PluginMetadata::new(
            &plugin_id,
            &plugin_id,
            "0.1.0",
        );

        // Get exported functions as available actions
        let actions: Vec<String> = module
            .exports()
            .filter_map(|e| {
                if e.ty().func().is_some() {
                    Some(e.name().to_string())
                } else {
                    None
                }
            })
            .filter(|name| !name.starts_with('_')) // Skip internal functions
            .collect();

        debug!("Plugin '{}' exports actions: {:?}", plugin_id, actions);

        let instance = PluginInstance {
            config,
            metadata,
            module,
            sandbox,
            actions,
        };

        self.plugins.insert(plugin_id.clone(), instance);
        info!("Plugin '{}' loaded successfully", plugin_id);

        Ok(plugin_id)
    }

    /// Load a plugin from a path with default configuration
    pub fn load_plugin_from_path(&mut self, path: impl Into<PathBuf>) -> Result<String> {
        let config = PluginConfig::new(path);
        self.load_plugin(config)
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, id: &str) -> Result<()> {
        if self.plugins.remove(id).is_some() {
            info!("Plugin '{}' unloaded", id);
            Ok(())
        } else {
            Err(PluginError::NotFound(id.to_string()))
        }
    }

    /// Get a plugin by ID
    pub fn get_plugin(&self, id: &str) -> Option<&PluginInstance> {
        self.plugins.get(id)
    }

    /// Get all loaded plugins
    pub fn plugins(&self) -> impl Iterator<Item = &PluginInstance> {
        self.plugins.values()
    }

    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Execute an action on a plugin
    pub fn execute_action(
        &self,
        plugin_id: &str,
        action: &str,
        ctx: &PluginContext,
    ) -> Result<PluginActionResult> {
        let plugin = self
            .get_plugin(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        plugin.execute(action, ctx)
    }

    /// Find plugins that provide a specific action
    pub fn find_plugins_with_action(&self, action: &str) -> Vec<&PluginInstance> {
        self.plugins
            .values()
            .filter(|p| p.has_action(action))
            .collect()
    }

    /// Discover and load plugins from search paths
    pub fn discover_plugins(&mut self) -> Result<Vec<String>> {
        let mut loaded = Vec::new();

        for search_path in self.search_paths.clone() {
            if !search_path.exists() {
                debug!("Plugin search path does not exist: {}", search_path.display());
                continue;
            }

            let entries = std::fs::read_dir(&search_path)?;

            for entry in entries.flatten() {
                let path = entry.path();

                if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                    match self.load_plugin_from_path(&path) {
                        Ok(id) => loaded.push(id),
                        Err(e) => warn!("Failed to load plugin {}: {}", path.display(), e),
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Reload a plugin
    pub fn reload_plugin(&mut self, id: &str) -> Result<()> {
        let config = self
            .plugins
            .get(id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?
            .config
            .clone();

        self.unload_plugin(id)?;
        self.load_plugin(config)?;

        Ok(())
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new().expect("Failed to create plugin host")
    }
}

/// Registry for plugin actions that can be used in workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginActionConfig {
    /// Plugin ID
    pub plugin: String,
    /// Action name
    pub action: String,
    /// Action-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

impl PluginActionConfig {
    /// Create a new plugin action configuration
    pub fn new(plugin: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            plugin: plugin.into(),
            action: action.into(),
            config: HashMap::new(),
        }
    }

    /// Add configuration value
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_config() {
        let config = PluginConfig::new("/path/to/plugin.wasm")
            .with_id("my-plugin")
            .with_memory_limit(32 * 1024 * 1024)
            .with_permission(Permission::read_path("/tmp"));

        assert_eq!(config.get_id(), "my-plugin");
        assert!(config.enabled);
    }

    #[test]
    fn test_plugin_host_creation() {
        let host = PluginHost::new();
        assert!(host.is_ok());
    }
}
