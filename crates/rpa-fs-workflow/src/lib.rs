// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2024-2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! RPA Filesystem Workflow - Automated file operations based on events
//!
//! This crate provides filesystem automation capabilities:
//! - Watch directories for file changes
//! - Execute actions based on file events (create, modify, delete, rename)
//! - Supported actions: copy, move, archive, delete, rename patterns

#![forbid(unsafe_code)]
pub mod actions;
pub mod config;
pub mod runner;
pub mod watcher;

pub use config::WorkflowConfig;
pub use runner::WorkflowRunner;
pub use watcher::FsWatcher;
