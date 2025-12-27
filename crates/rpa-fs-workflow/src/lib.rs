// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! RPA Filesystem Workflow - Automated file operations based on events
//!
//! This crate provides filesystem automation capabilities:
//! - Watch directories for file changes
//! - Execute actions based on file events (create, modify, delete, rename)
//! - Supported actions: copy, move, archive, delete, rename patterns

pub mod actions;
pub mod config;
pub mod watcher;
pub mod runner;

pub use config::WorkflowConfig;
pub use runner::WorkflowRunner;
pub use watcher::FsWatcher;
