// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Plugin error types

use thiserror::Error;

/// Plugin-specific error type
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin load failed: {0}")]
    LoadFailed(String),

    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    #[error("Timeout: plugin execution exceeded {0}ms")]
    Timeout(u64),

    #[error("Invalid plugin format: {0}")]
    InvalidFormat(String),

    #[error("Sandbox error: {0}")]
    SandboxError(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WASM error: {0}")]
    Wasm(String),

    #[error("Plugin API version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: String, got: String },

    #[error("{0}")]
    Other(String),
}

impl From<wasmtime::Error> for PluginError {
    fn from(e: wasmtime::Error) -> Self {
        PluginError::Wasm(e.to_string())
    }
}

/// Result type alias for plugin operations
pub type Result<T> = std::result::Result<T, PluginError>;
