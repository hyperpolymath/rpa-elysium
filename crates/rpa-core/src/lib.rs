// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! RPA Core - Foundation types and traits for RPA Elysium
//!
//! This crate provides the core abstractions used across all RPA Elysium modules:
//! - Event types for workflow triggers
//! - Action traits for automation operations
//! - Result and error types
//! - State management interfaces

pub mod event;
pub mod action;
pub mod workflow;
pub mod error;

pub use error::{Error, Result};
pub use event::{Event, EventKind};
pub use action::Action;
pub use workflow::{Workflow, WorkflowState};
