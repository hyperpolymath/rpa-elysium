// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2024-2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! RPA Core - Foundation types and traits for RPA Elysium
//!
//! This crate provides the core abstractions used across all RPA Elysium modules:
//! - Event types for workflow triggers
//! - Action traits for automation operations
//! - Result and error types
//! - State management interfaces

#![forbid(unsafe_code)]
pub mod action;
pub mod error;
pub mod event;
pub mod workflow;

pub use action::Action;
pub use error::{Error, Result};
pub use event::{Event, EventKind};
pub use workflow::{Workflow, WorkflowState};
