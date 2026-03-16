// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Workflow state snapshots
//!
//! A [`Snapshot`] captures the full state of a workflow at a point in time,
//! including execution counters and arbitrary metadata. Snapshots are
//! serialisable to/from JSON for persistence via any [`StateBackend`].

use anyhow::Result;
use chrono::{DateTime, Utc};
use rpa_core::WorkflowState;
use serde::{Deserialize, Serialize};

/// A point-in-time capture of a workflow's execution state.
///
/// Snapshots record the workflow name, the full [`WorkflowState`], execution
/// counters, and an optional free-form metadata blob. They are designed
/// to be serialised as JSON and stored via a [`StateBackend`](crate::StateBackend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Name of the workflow this snapshot belongs to.
    pub workflow_name: String,

    /// Timestamp when this snapshot was taken.
    pub timestamp: DateTime<Utc>,

    /// The full workflow state at snapshot time.
    pub state: WorkflowState,

    /// Total number of events processed up to this snapshot.
    pub events_processed: u64,

    /// Total number of actions executed up to this snapshot.
    pub actions_executed: u64,

    /// Total number of errors encountered up to this snapshot.
    pub error_count: u64,

    /// Arbitrary metadata attached to this snapshot.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Snapshot {
    /// Create a snapshot from the current state of a workflow.
    ///
    /// Copies the execution counters directly from [`WorkflowState`] and
    /// initialises metadata to `null`.
    pub fn from_workflow_state(name: &str, state: &WorkflowState) -> Self {
        Self {
            workflow_name: name.to_string(),
            timestamp: Utc::now(),
            state: state.clone(),
            events_processed: state.events_processed,
            actions_executed: state.actions_executed,
            error_count: state.error_count,
            metadata: serde_json::Value::Null,
        }
    }

    /// Serialise this snapshot to a JSON string.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialise a snapshot from a JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        let snapshot: Snapshot = serde_json::from_str(json)?;
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_roundtrip() {
        let mut ws = WorkflowState::new("roundtrip-test");
        ws.start();
        ws.record_event();
        ws.record_event();
        ws.record_action();
        ws.record_error();

        let snap = Snapshot::from_workflow_state("roundtrip-test", &ws);
        assert_eq!(snap.workflow_name, "roundtrip-test");
        assert_eq!(snap.events_processed, 2);
        assert_eq!(snap.actions_executed, 1);
        assert_eq!(snap.error_count, 1);

        // Round-trip through JSON
        let json = snap.to_json().expect("to_json failed");
        let restored = Snapshot::from_json(&json).expect("from_json failed");

        assert_eq!(restored.workflow_name, snap.workflow_name);
        assert_eq!(restored.events_processed, snap.events_processed);
        assert_eq!(restored.actions_executed, snap.actions_executed);
        assert_eq!(restored.error_count, snap.error_count);
        assert_eq!(restored.state.workflow_name, snap.state.workflow_name);
    }

    #[test]
    fn test_snapshot_metadata() {
        let ws = WorkflowState::new("meta-test");
        let mut snap = Snapshot::from_workflow_state("meta-test", &ws);
        snap.metadata = serde_json::json!({"version": "1.0", "note": "test snapshot"});

        let json = snap.to_json().expect("to_json failed");
        let restored = Snapshot::from_json(&json).expect("from_json failed");
        assert_eq!(restored.metadata["version"], "1.0");
        assert_eq!(restored.metadata["note"], "test snapshot");
    }
}
