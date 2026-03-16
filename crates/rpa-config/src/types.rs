// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Core types for the configuration subsystem
//!
//! Defines the supported configuration formats and the crate-level error type
//! that wraps [`rpa_core::Error`] with config-specific variants.

use std::path::Path;
use thiserror::Error;

/// Supported configuration file formats.
///
/// Determined by file extension in [`ConfigFormat::detect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    /// Standard JSON (`.json` extension)
    Json,
    /// Nickel configuration language (`.ncl` extension)
    Nickel,
    /// Unrecognised extension
    Unknown,
}

impl ConfigFormat {
    /// Detect format from a file path's extension.
    ///
    /// Returns [`ConfigFormat::Unknown`] when the extension is missing or
    /// not one of the recognised values.
    pub fn detect(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Self::Json,
            Some("ncl") => Self::Nickel,
            _ => Self::Unknown,
        }
    }
}

/// Configuration-specific errors.
///
/// These extend the broader [`rpa_core::Error`] with detail relevant to
/// the config loading and validation pipeline.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// The configuration file could not be read from disk.
    #[error("Cannot read config file '{path}': {source}")]
    ReadFailed {
        path: String,
        source: std::io::Error,
    },

    /// JSON parsing failed — includes line and column when available.
    #[error("Invalid JSON in '{path}': {message}")]
    JsonParse { path: String, message: String },

    /// Nickel evaluation failed (CLI returned non-zero).
    #[error("Nickel evaluation failed: {0}")]
    NickelEval(String),

    /// The `nickel` binary is not installed or not on `$PATH`.
    #[error("Nickel CLI not found: {0}. Ensure nickel is installed.")]
    NickelNotFound(String),

    /// Unsupported file format (neither `.json` nor `.ncl`).
    #[error("Unsupported config format: {0}. Use .json or .ncl")]
    UnsupportedFormat(String),

    /// One or more validation checks failed.
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Passthrough for the core error type.
    #[error(transparent)]
    Core(#[from] rpa_core::Error),
}

/// Convenience result type for config operations.
pub type Result<T> = std::result::Result<T, ConfigError>;
