// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Configuration loading and validation for RPA Elysium
//!
//! Provides a generic configuration subsystem that supports multiple formats:
//! - **JSON** — parsed with detailed error messages (line/column on failure)
//! - **Nickel** — evaluated via the `nickel` CLI and converted to JSON
//!
//! The crate also includes a pluggable validation framework built on the
//! [`Validator`] trait, with built-in validators for required fields and
//! type checking.

#![forbid(unsafe_code)]
pub mod loader;
pub mod nickel;
pub mod types;
pub mod validate;

pub use loader::ConfigLoader;
pub use types::{ConfigError, ConfigFormat};
pub use validate::{ValidationResult, Validator};
