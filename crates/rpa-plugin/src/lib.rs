// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! RPA Plugin - Plugin API and WASM Sandbox for RPA Elysium
//!
//! This crate provides a secure plugin system for extending RPA Elysium:
//!
//! - **Plugin API**: Traits and types for writing plugins
//! - **WASM Sandbox**: Secure execution environment using WebAssembly
//! - **Permission System**: Fine-grained control over plugin capabilities
//! - **Resource Limits**: Memory, CPU, and I/O constraints
//!
//! # Security Model
//!
//! Plugins run in isolated WASM sandboxes with:
//! - No direct filesystem access (must request through host)
//! - No network access (must request through host)
//! - Memory limits (configurable, default 64MB)
//! - Execution time limits (configurable, default 30s)
//! - Explicit permission grants for each capability
//!
//! # Example
//!
//! ```ignore
//! use rpa_plugin::{PluginHost, PluginConfig, Permission};
//!
//! let mut host = PluginHost::new()?;
//!
//! // Load a plugin with specific permissions
//! let config = PluginConfig::new("my-plugin.wasm")
//!     .with_permission(Permission::ReadPath("/tmp/input".into()))
//!     .with_memory_limit(32 * 1024 * 1024); // 32MB
//!
//! host.load_plugin(config)?;
//! ```

pub mod api;
pub mod error;
pub mod host;
pub mod permissions;
pub mod sandbox;

pub use api::{Plugin, PluginAction, PluginContext, PluginMetadata};
pub use error::{PluginError, Result};
pub use host::{PluginHost, PluginInstance};
pub use permissions::{Permission, PermissionSet};
pub use sandbox::{Sandbox, SandboxConfig};
