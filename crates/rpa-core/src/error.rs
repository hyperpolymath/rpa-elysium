// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Error types for RPA operations

use thiserror::Error;

/// Core error type for RPA operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Workflow error: {0}")]
    Workflow(String),

    #[error("Action failed: {action} - {reason}")]
    ActionFailed { action: String, reason: String },

    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),

    #[error("Watch error: {0}")]
    Watch(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias for RPA operations
pub type Result<T> = std::result::Result<T, Error>;
