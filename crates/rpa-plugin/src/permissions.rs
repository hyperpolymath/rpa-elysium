// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Plugin permission system
//!
//! Provides fine-grained control over what plugins can access.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Individual permission grant
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Permission {
    /// Read access to a specific path (file or directory)
    ReadPath { path: PathBuf },

    /// Write access to a specific path (file or directory)
    WritePath { path: PathBuf },

    /// Access to specific environment variable
    Env { name: String },

    /// Access to all environment variables
    AllEnv,

    /// Network access to specific host:port
    Network { host: String, port: Option<u16> },

    /// Execute external commands (dangerous!)
    Execute { command: String },

    /// Access to current time
    Time,

    /// Access to random/UUID generation
    Random,
}

impl Permission {
    /// Create a read path permission
    pub fn read_path(path: impl Into<PathBuf>) -> Self {
        Self::ReadPath { path: path.into() }
    }

    /// Create a write path permission
    pub fn write_path(path: impl Into<PathBuf>) -> Self {
        Self::WritePath { path: path.into() }
    }

    /// Create an environment variable permission
    pub fn env(name: impl Into<String>) -> Self {
        Self::Env { name: name.into() }
    }

    /// Create a network permission
    pub fn network(host: impl Into<String>, port: Option<u16>) -> Self {
        Self::Network {
            host: host.into(),
            port,
        }
    }

    /// Check if this permission covers the requested access
    pub fn covers(&self, requested: &Permission) -> bool {
        match (self, requested) {
            // Exact match
            (a, b) if a == b => true,

            // ReadPath covers if granted path is parent of requested
            (
                Permission::ReadPath { path: granted },
                Permission::ReadPath { path: requested },
            ) => path_covers(granted, requested),

            // WritePath covers if granted path is parent of requested
            (
                Permission::WritePath { path: granted },
                Permission::WritePath { path: requested },
            ) => path_covers(granted, requested),

            // AllEnv covers any Env
            (Permission::AllEnv, Permission::Env { .. }) => true,

            // Network with no port covers any port on that host
            (
                Permission::Network {
                    host: h1,
                    port: None,
                },
                Permission::Network { host: h2, .. },
            ) => h1 == h2,

            _ => false,
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            Permission::ReadPath { path } => format!("read {}", path.display()),
            Permission::WritePath { path } => format!("write {}", path.display()),
            Permission::Env { name } => format!("env ${}", name),
            Permission::AllEnv => "all environment variables".to_string(),
            Permission::Network { host, port } => {
                if let Some(p) = port {
                    format!("network {}:{}", host, p)
                } else {
                    format!("network {}", host)
                }
            }
            Permission::Execute { command } => format!("execute {}", command),
            Permission::Time => "current time".to_string(),
            Permission::Random => "random/UUID generation".to_string(),
        }
    }
}

/// Check if granted path covers requested path
fn path_covers(granted: &Path, requested: &Path) -> bool {
    // Normalize paths for comparison
    let granted = granted.canonicalize().unwrap_or_else(|_| granted.to_path_buf());
    let requested = requested.canonicalize().unwrap_or_else(|_| requested.to_path_buf());

    // Exact match or granted is parent
    requested == granted || requested.starts_with(&granted)
}

/// Set of permissions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSet {
    permissions: HashSet<Permission>,
}

impl PermissionSet {
    /// Create an empty permission set
    pub fn empty() -> Self {
        Self {
            permissions: HashSet::new(),
        }
    }

    /// Create a permission set with given permissions
    pub fn new(perms: impl IntoIterator<Item = Permission>) -> Self {
        Self {
            permissions: perms.into_iter().collect(),
        }
    }

    /// Add a permission
    pub fn add(&mut self, perm: Permission) {
        self.permissions.insert(perm);
    }

    /// Add a permission (builder pattern)
    pub fn with(mut self, perm: Permission) -> Self {
        self.add(perm);
        self
    }

    /// Check if a permission is granted
    pub fn check(&self, requested: &Permission) -> bool {
        self.permissions.iter().any(|p| p.covers(requested))
    }

    /// Check if all requested permissions are granted
    pub fn check_all(&self, requested: &PermissionSet) -> bool {
        requested.permissions.iter().all(|p| self.check(p))
    }

    /// Get missing permissions
    pub fn missing(&self, requested: &PermissionSet) -> Vec<Permission> {
        requested
            .permissions
            .iter()
            .filter(|p| !self.check(p))
            .cloned()
            .collect()
    }

    /// Check if set is empty
    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }

    /// Get number of permissions
    pub fn len(&self) -> usize {
        self.permissions.len()
    }

    /// Iterate over permissions
    pub fn iter(&self) -> impl Iterator<Item = &Permission> {
        self.permissions.iter()
    }
}

impl FromIterator<Permission> for PermissionSet {
    fn from_iter<T: IntoIterator<Item = Permission>>(iter: T) -> Self {
        Self::new(iter)
    }
}

/// Permission check result
#[derive(Debug, Clone)]
pub struct PermissionCheck {
    pub granted: bool,
    pub permission: Permission,
    pub reason: Option<String>,
}

impl PermissionCheck {
    pub fn allowed(permission: Permission) -> Self {
        Self {
            granted: true,
            permission,
            reason: None,
        }
    }

    pub fn denied(permission: Permission, reason: impl Into<String>) -> Self {
        Self {
            granted: false,
            permission,
            reason: Some(reason.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_permission_coverage() {
        let granted = Permission::read_path("/home/user/data");
        let requested = Permission::read_path("/home/user/data/file.txt");

        assert!(granted.covers(&requested));
    }

    #[test]
    fn test_permission_set() {
        let set = PermissionSet::empty()
            .with(Permission::read_path("/tmp"))
            .with(Permission::Time)
            .with(Permission::Random);

        assert!(set.check(&Permission::read_path("/tmp/file.txt")));
        assert!(set.check(&Permission::Time));
        assert!(!set.check(&Permission::write_path("/tmp")));
    }

    #[test]
    fn test_all_env_covers_specific() {
        let set = PermissionSet::empty().with(Permission::AllEnv);

        assert!(set.check(&Permission::env("HOME")));
        assert!(set.check(&Permission::env("PATH")));
    }
}
