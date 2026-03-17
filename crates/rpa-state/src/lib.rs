// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! RPA State - Persistence layer for RPA Elysium
//!
//! This crate provides state persistence capabilities for workflow execution,
//! including snapshot management and pluggable storage backends.
//!
//! # Modules
//!
//! - [`store`] - Main persistence interface (`StateStore`)
//! - [`backend`] - Storage backend trait and implementations (`StateBackend`, `JsonFileBackend`)
//! - [`snapshot`] - Workflow state snapshots (`Snapshot`)

#![forbid(unsafe_code)]
pub mod backend;
pub mod snapshot;
pub mod store;

pub use backend::{JsonFileBackend, StateBackend};
pub use snapshot::Snapshot;
pub use store::StateStore;
