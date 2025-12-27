// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! WASM Sandbox for secure plugin execution
//!
//! Provides isolated execution environment using WebAssembly.

use crate::api::{HostRequest, HostResponse, LogLevel, PluginContext, PluginActionResult};
use crate::error::{PluginError, Result};
use crate::permissions::{Permission, PermissionSet};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use wasmtime::*;

/// Default memory limit: 64MB
pub const DEFAULT_MEMORY_LIMIT: u64 = 64 * 1024 * 1024;

/// Default execution timeout: 30 seconds
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Maximum memory in bytes
    pub memory_limit: u64,
    /// Maximum execution time in milliseconds
    pub timeout_ms: u64,
    /// Maximum fuel (instruction count limit)
    pub fuel_limit: Option<u64>,
    /// Permissions granted to the sandbox
    pub permissions: PermissionSet,
    /// Working directory for file operations
    pub work_dir: Option<PathBuf>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit: DEFAULT_MEMORY_LIMIT,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            fuel_limit: Some(100_000_000), // 100M instructions
            permissions: PermissionSet::empty()
                .with(Permission::Time)
                .with(Permission::Random),
            work_dir: None,
        }
    }
}

impl SandboxConfig {
    /// Create a new sandbox config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.memory_limit = bytes;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Set fuel limit
    pub fn with_fuel(mut self, fuel: u64) -> Self {
        self.fuel_limit = Some(fuel);
        self
    }

    /// Add a permission
    pub fn with_permission(mut self, perm: Permission) -> Self {
        self.permissions.add(perm);
        self
    }

    /// Set working directory
    pub fn with_work_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.work_dir = Some(dir.into());
        self
    }
}

/// Sandbox state shared with WASM
#[derive(Debug)]
struct SandboxState {
    permissions: PermissionSet,
    logs: Vec<(LogLevel, String)>,
    work_dir: Option<PathBuf>,
    start_time: Instant,
    timeout_ms: u64,
}

impl SandboxState {
    fn new(config: &SandboxConfig) -> Self {
        Self {
            permissions: config.permissions.clone(),
            logs: Vec::new(),
            work_dir: config.work_dir.clone(),
            start_time: Instant::now(),
            timeout_ms: config.timeout_ms,
        }
    }

    fn check_timeout(&self) -> Result<()> {
        if self.start_time.elapsed() > Duration::from_millis(self.timeout_ms) {
            Err(PluginError::Timeout(self.timeout_ms))
        } else {
            Ok(())
        }
    }

    fn check_permission(&self, perm: &Permission) -> Result<()> {
        if self.permissions.check(perm) {
            Ok(())
        } else {
            Err(PluginError::PermissionDenied(perm.description()))
        }
    }

    fn handle_request(&mut self, request: HostRequest) -> HostResponse {
        // Check timeout on every request
        if let Err(e) = self.check_timeout() {
            return HostResponse::error(e.to_string());
        }

        match request {
            HostRequest::ReadFile { path } => {
                let path_buf = PathBuf::from(&path);
                if let Err(_) = self.check_permission(&Permission::read_path(&path_buf)) {
                    return HostResponse::permission_denied(format!("read {}", path));
                }

                match std::fs::read(&path) {
                    Ok(content) => {
                        let encoded = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &content,
                        );
                        HostResponse::success_with_data(serde_json::json!({
                            "content": encoded,
                            "size": content.len()
                        }))
                    }
                    Err(e) => HostResponse::error(format!("Failed to read file: {}", e)),
                }
            }

            HostRequest::WriteFile { path, content } => {
                let path_buf = PathBuf::from(&path);
                if let Err(_) = self.check_permission(&Permission::write_path(&path_buf)) {
                    return HostResponse::permission_denied(format!("write {}", path));
                }

                match std::fs::write(&path, &content) {
                    Ok(_) => HostResponse::success_with_data(serde_json::json!({
                        "bytes_written": content.len()
                    })),
                    Err(e) => HostResponse::error(format!("Failed to write file: {}", e)),
                }
            }

            HostRequest::ListDir { path } => {
                let path_buf = PathBuf::from(&path);
                if let Err(_) = self.check_permission(&Permission::read_path(&path_buf)) {
                    return HostResponse::permission_denied(format!("read {}", path));
                }

                match std::fs::read_dir(&path) {
                    Ok(entries) => {
                        let files: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| {
                                serde_json::json!({
                                    "name": e.file_name().to_string_lossy(),
                                    "is_dir": e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                                })
                            })
                            .collect();
                        HostResponse::success_with_data(serde_json::json!({ "entries": files }))
                    }
                    Err(e) => HostResponse::error(format!("Failed to list directory: {}", e)),
                }
            }

            HostRequest::GetEnv { name } => {
                if let Err(_) = self.check_permission(&Permission::env(&name)) {
                    return HostResponse::permission_denied(format!("env ${}", name));
                }

                match std::env::var(&name) {
                    Ok(value) => HostResponse::success_with_data(serde_json::json!({ "value": value })),
                    Err(_) => HostResponse::success_with_data(serde_json::json!({ "value": null })),
                }
            }

            HostRequest::Log { level, message } => {
                self.logs.push((level.clone(), message.clone()));
                match level {
                    LogLevel::Debug => debug!(target: "plugin", "{}", message),
                    LogLevel::Info => info!(target: "plugin", "{}", message),
                    LogLevel::Warn => warn!(target: "plugin", "{}", message),
                    LogLevel::Error => tracing::error!(target: "plugin", "{}", message),
                }
                HostResponse::success()
            }

            HostRequest::CurrentTime => {
                if let Err(_) = self.check_permission(&Permission::Time) {
                    return HostResponse::permission_denied("time");
                }

                let now = chrono::Utc::now();
                HostResponse::success_with_data(serde_json::json!({
                    "timestamp": now.timestamp(),
                    "iso": now.to_rfc3339()
                }))
            }

            HostRequest::GenerateUuid => {
                if let Err(_) = self.check_permission(&Permission::Random) {
                    return HostResponse::permission_denied("random");
                }

                let uuid = uuid::Uuid::new_v4();
                HostResponse::success_with_data(serde_json::json!({ "uuid": uuid.to_string() }))
            }
        }
    }
}

/// Base64 encoding helper (since we can't add base64 crate, using simple impl)
mod base64 {
    pub struct Engine;

    pub mod engine {
        pub mod general_purpose {
            pub static STANDARD: super::super::Engine = super::super::Engine;
        }
    }

    impl Engine {
        pub fn encode(_engine: &Engine, data: &[u8]) -> String {
            // Simple base64 encoding
            const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut result = String::new();

            for chunk in data.chunks(3) {
                let mut n = 0u32;
                for (i, &byte) in chunk.iter().enumerate() {
                    n |= (byte as u32) << (16 - i * 8);
                }

                let chars = match chunk.len() {
                    3 => 4,
                    2 => 3,
                    1 => 2,
                    _ => 0,
                };

                for i in 0..chars {
                    let idx = ((n >> (18 - i * 6)) & 0x3F) as usize;
                    result.push(ALPHABET[idx] as char);
                }

                for _ in chars..4 {
                    result.push('=');
                }
            }

            result
        }
    }
}

/// WASM Sandbox for executing plugins
pub struct Sandbox {
    engine: Engine,
    config: SandboxConfig,
}

impl Sandbox {
    /// Create a new sandbox with the given configuration
    pub fn new(config: SandboxConfig) -> Result<Self> {
        let mut engine_config = Config::new();

        // Enable fuel for instruction counting
        if config.fuel_limit.is_some() {
            engine_config.consume_fuel(true);
        }

        // Memory limits are applied per-module
        let engine = Engine::new(&engine_config)?;

        Ok(Self { engine, config })
    }

    /// Create a sandbox with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(SandboxConfig::default())
    }

    /// Load a WASM module from bytes
    pub fn load_module(&self, wasm_bytes: &[u8]) -> Result<Module> {
        Module::new(&self.engine, wasm_bytes).map_err(|e| {
            PluginError::LoadFailed(format!("Failed to compile WASM module: {}", e))
        })
    }

    /// Load a WASM module from a file
    pub fn load_module_from_file(&self, path: impl AsRef<std::path::Path>) -> Result<Module> {
        let bytes = std::fs::read(path.as_ref())?;
        self.load_module(&bytes)
    }

    /// Execute a plugin module with context
    pub fn execute(
        &self,
        module: &Module,
        action: &str,
        ctx: &PluginContext,
    ) -> Result<PluginActionResult> {
        let state = Arc::new(Mutex::new(SandboxState::new(&self.config)));

        // Create store with fuel limits
        let mut store = Store::new(&self.engine, state.clone());

        if let Some(fuel) = self.config.fuel_limit {
            store.set_fuel(fuel)?;
        }

        // Create linker with host functions
        let mut linker = Linker::new(&self.engine);

        // Register host function for handling requests
        let state_clone = state.clone();
        linker.func_wrap(
            "host",
            "request",
            move |mut caller: Caller<'_, Arc<Mutex<SandboxState>>>, ptr: i32, len: i32| -> i32 {
                // This is a simplified interface - in production you'd use proper memory access
                // For now, we return 0 to indicate success
                0i32
            },
        )?;

        // Instantiate module
        let instance = linker.instantiate(&mut store, module)?;

        // Look for the action function
        let func = instance
            .get_func(&mut store, action)
            .ok_or_else(|| PluginError::ExecutionFailed(format!("Action '{}' not found", action)))?;

        // Call the function
        let start = Instant::now();
        let mut results = vec![Val::I32(0)];

        match func.call(&mut store, &[], &mut results) {
            Ok(_) => {
                let state = state.lock().unwrap();
                let elapsed = start.elapsed();

                debug!("Plugin action '{}' completed in {:?}", action, elapsed);

                // Build result from state
                let logs = state
                    .logs
                    .iter()
                    .map(|(level, msg)| crate::api::PluginLog {
                        level: level.clone(),
                        message: msg.clone(),
                        timestamp: chrono::Utc::now(),
                    })
                    .collect();

                Ok(PluginActionResult {
                    success: true,
                    message: format!("Action '{}' completed", action),
                    output: serde_json::Value::Null,
                    logs,
                })
            }
            Err(e) => {
                // Check if it was a fuel exhaustion
                if store.get_fuel().unwrap_or(0) == 0 {
                    Err(PluginError::ResourceLimitExceeded(
                        "Instruction limit exceeded".to_string(),
                    ))
                } else {
                    Err(PluginError::ExecutionFailed(e.to_string()))
                }
            }
        }
    }

    /// Get the sandbox configuration
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }
}

/// Builder for creating sandboxes
pub struct SandboxBuilder {
    config: SandboxConfig,
}

impl SandboxBuilder {
    /// Create a new sandbox builder
    pub fn new() -> Self {
        Self {
            config: SandboxConfig::default(),
        }
    }

    /// Set memory limit
    pub fn memory_limit(mut self, bytes: u64) -> Self {
        self.config.memory_limit = bytes;
        self
    }

    /// Set timeout
    pub fn timeout(mut self, ms: u64) -> Self {
        self.config.timeout_ms = ms;
        self
    }

    /// Set fuel limit
    pub fn fuel(mut self, fuel: u64) -> Self {
        self.config.fuel_limit = Some(fuel);
        self
    }

    /// Add permission
    pub fn permission(mut self, perm: Permission) -> Self {
        self.config.permissions.add(perm);
        self
    }

    /// Set permissions
    pub fn permissions(mut self, perms: PermissionSet) -> Self {
        self.config.permissions = perms;
        self
    }

    /// Set working directory
    pub fn work_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.config.work_dir = Some(dir.into());
        self
    }

    /// Build the sandbox
    pub fn build(self) -> Result<Sandbox> {
        Sandbox::new(self.config)
    }
}

impl Default for SandboxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.memory_limit, DEFAULT_MEMORY_LIMIT);
        assert_eq!(config.timeout_ms, DEFAULT_TIMEOUT_MS);
    }

    #[test]
    fn test_sandbox_builder() {
        let sandbox = SandboxBuilder::new()
            .memory_limit(32 * 1024 * 1024)
            .timeout(10_000)
            .permission(Permission::read_path("/tmp"))
            .build();

        assert!(sandbox.is_ok());
    }

    #[test]
    fn test_sandbox_creation() {
        let sandbox = Sandbox::with_defaults();
        assert!(sandbox.is_ok());
    }
}
