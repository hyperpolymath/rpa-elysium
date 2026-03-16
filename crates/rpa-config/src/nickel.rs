// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Nickel configuration bridge
//!
//! Evaluates `.ncl` files via the `nickel` CLI and returns the result as a
//! JSON string. This avoids linking `nickel-lang-core` at runtime while
//! still supporting the Nickel configuration language.

use crate::types::{ConfigError, Result};
use std::path::Path;
use tracing::debug;

/// Bridge to the Nickel configuration language.
///
/// Uses the `nickel` CLI binary to evaluate Nickel source files and export
/// them as JSON. The binary must be installed and available on `$PATH`.
#[derive(Debug, Default)]
pub struct NickelBridge;

impl NickelBridge {
    /// Create a new bridge instance.
    pub fn new() -> Self {
        Self
    }

    /// Evaluate a Nickel file and return the resulting JSON string.
    ///
    /// Runs `nickel export --format json <path>` and captures stdout.
    /// Returns a [`ConfigError::NickelNotFound`] if the binary is missing,
    /// or [`ConfigError::NickelEval`] if evaluation fails.
    pub fn evaluate(&self, path: &Path) -> Result<String> {
        debug!("Evaluating Nickel file: {}", path.display());

        let output = std::process::Command::new("nickel")
            .args(["export", "--format", "json"])
            .arg(path)
            .output()
            .map_err(|e| ConfigError::NickelNotFound(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ConfigError::NickelEval(stderr.into_owned()));
        }

        let json = String::from_utf8_lossy(&output.stdout).into_owned();
        Ok(json)
    }

    /// Check whether the `nickel` CLI is installed and reachable.
    ///
    /// Returns `true` if `nickel --version` exits successfully.
    pub fn is_available() -> bool {
        std::process::Command::new("nickel")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
