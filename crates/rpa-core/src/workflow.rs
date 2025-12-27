// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Workflow types and state management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique name for this workflow
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Whether the workflow is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Version of the workflow
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_enabled() -> bool {
    true
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl Workflow {
    /// Create a new workflow
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            enabled: true,
            version: "1.0.0".to_string(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Current state of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Name of the workflow
    pub workflow_name: String,
    /// Current status
    pub status: WorkflowStatus,
    /// When the workflow started
    pub started_at: Option<DateTime<Utc>>,
    /// When the workflow completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of events processed
    pub events_processed: u64,
    /// Number of actions executed
    pub actions_executed: u64,
    /// Number of errors encountered
    pub error_count: u64,
}

/// Status of a workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Workflow is idle, waiting for events
    Idle,
    /// Workflow is running
    Running,
    /// Workflow is paused
    Paused,
    /// Workflow has stopped
    Stopped,
    /// Workflow encountered an error
    Error,
}

impl WorkflowState {
    /// Create a new workflow state
    pub fn new(workflow_name: impl Into<String>) -> Self {
        Self {
            workflow_name: workflow_name.into(),
            status: WorkflowStatus::Idle,
            started_at: None,
            completed_at: None,
            events_processed: 0,
            actions_executed: 0,
            error_count: 0,
        }
    }

    /// Mark as running
    pub fn start(&mut self) {
        self.status = WorkflowStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Mark as stopped
    pub fn stop(&mut self) {
        self.status = WorkflowStatus::Stopped;
        self.completed_at = Some(Utc::now());
    }

    /// Increment events processed
    pub fn record_event(&mut self) {
        self.events_processed += 1;
    }

    /// Increment actions executed
    pub fn record_action(&mut self) {
        self.actions_executed += 1;
    }

    /// Increment error count
    pub fn record_error(&mut self) {
        self.error_count += 1;
    }
}
