// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Main persistence interface
//!
//! [`StateStore`] wraps a [`StateBackend`] and provides high-level methods for
//! saving and loading workflow states and snapshots. It handles serialisation
//! and key-space partitioning so callers work with domain types directly.

use anyhow::Result;
use rpa_core::WorkflowState;
use tracing::{debug, instrument};

use crate::backend::StateBackend;
use crate::snapshot::Snapshot;

/// Key prefix for workflow state entries.
const WORKFLOW_PREFIX: &str = "workflow/";

/// Key prefix for snapshot entries.
const SNAPSHOT_PREFIX: &str = "snapshot/";

/// High-level persistence interface for workflow states and snapshots.
///
/// Delegates all storage to a pluggable [`StateBackend`]. Workflow states and
/// snapshots are kept in separate key-space prefixes to avoid collisions.
pub struct StateStore {
    /// The underlying storage backend.
    backend: Box<dyn StateBackend>,
}

impl StateStore {
    /// Create a new `StateStore` backed by the given backend.
    pub fn new(backend: Box<dyn StateBackend>) -> Self {
        Self { backend }
    }

    /// Persist a workflow's current state under the given name.
    ///
    /// Serialises `state` to JSON and writes it to key `workflow/<name>`.
    #[instrument(skip(self, state), fields(store_backend = %self.backend.name()))]
    pub async fn save_workflow_state(&self, name: &str, state: &WorkflowState) -> Result<()> {
        let json = serde_json::to_vec_pretty(state)?;
        let key = format!("{}{}", WORKFLOW_PREFIX, name);
        debug!(key = %key, "Saving workflow state");
        self.backend.put(&key, &json).await
    }

    /// Load a previously persisted workflow state by name.
    ///
    /// Returns `None` if no state has been saved for the given name.
    #[instrument(skip(self), fields(store_backend = %self.backend.name()))]
    pub async fn load_workflow_state(&self, name: &str) -> Result<Option<WorkflowState>> {
        let key = format!("{}{}", WORKFLOW_PREFIX, name);
        debug!(key = %key, "Loading workflow state");

        let maybe_data: Option<Vec<u8>> = self.backend.get(&key).await?;
        match maybe_data {
            Some(data) => {
                let state: WorkflowState = serde_json::from_slice(&data)?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// Persist a snapshot, keyed by its workflow name and timestamp.
    ///
    /// The key is `snapshot/<workflow_name>_<timestamp_millis>`, ensuring
    /// multiple snapshots for the same workflow are stored separately.
    #[instrument(skip(self, snapshot), fields(store_backend = %self.backend.name()))]
    pub async fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let ts = snapshot.timestamp.timestamp_millis();
        let key = format!("{}{}_{}", SNAPSHOT_PREFIX, snapshot.workflow_name, ts);
        let json = serde_json::to_vec_pretty(snapshot)?;
        debug!(key = %key, "Saving snapshot");
        self.backend.put(&key, &json).await
    }

    /// Load the most recent snapshot for the named workflow.
    ///
    /// Scans all snapshot keys matching `snapshot/<workflow_name>`, sorts them,
    /// and returns the latest one. Returns `None` if no snapshots exist.
    #[instrument(skip(self), fields(store_backend = %self.backend.name()))]
    pub async fn load_latest_snapshot(&self, workflow_name: &str) -> Result<Option<Snapshot>> {
        let prefix = format!("{}{}", SNAPSHOT_PREFIX, workflow_name);
        debug!(prefix = %prefix, "Loading latest snapshot");

        let mut keys: Vec<String> = self.backend.list_keys(&prefix).await?;
        keys.sort();

        // The last key (lexicographically) corresponds to the latest timestamp
        if let Some(latest_key) = keys.last() {
            let maybe_data: Option<Vec<u8>> = self.backend.get(latest_key).await?;
            match maybe_data {
                Some(data) => {
                    let snapshot: Snapshot = serde_json::from_slice(&data)?;
                    Ok(Some(snapshot))
                }
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// List the names of all persisted workflows.
    ///
    /// Returns just the workflow names (without the `workflow/` prefix).
    #[instrument(skip(self), fields(store_backend = %self.backend.name()))]
    pub async fn list_workflows(&self) -> Result<Vec<String>> {
        debug!("Listing workflows");
        let keys: Vec<String> = self.backend.list_keys(WORKFLOW_PREFIX).await?;
        let names: Vec<String> = keys
            .into_iter()
            .map(|k: String| k.strip_prefix(WORKFLOW_PREFIX).unwrap_or(&k).to_string())
            .collect();
        Ok(names)
    }

    /// Delete a workflow's persisted state.
    ///
    /// This removes only the workflow state entry; snapshots are not affected.
    #[instrument(skip(self), fields(store_backend = %self.backend.name()))]
    pub async fn delete_workflow_state(&self, name: &str) -> Result<()> {
        let key = format!("{}{}", WORKFLOW_PREFIX, name);
        debug!(key = %key, "Deleting workflow state");
        self.backend.delete(&key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::JsonFileBackend;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_state_store_save_load() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());
        let store = StateStore::new(Box::new(backend));

        let mut state = WorkflowState::new("test-workflow");
        state.start();
        state.record_event();
        state.record_action();

        // Save and reload
        store
            .save_workflow_state("test-workflow", &state)
            .await
            .expect("save failed");

        let loaded = store
            .load_workflow_state("test-workflow")
            .await
            .expect("load failed")
            .expect("state should exist");

        assert_eq!(loaded.workflow_name, "test-workflow");
        assert_eq!(loaded.events_processed, 1);
        assert_eq!(loaded.actions_executed, 1);
    }

    #[tokio::test]
    async fn test_state_store_load_missing() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());
        let store = StateStore::new(Box::new(backend));

        let result = store
            .load_workflow_state("nonexistent")
            .await
            .expect("load should not error");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_state_store_list_and_delete() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());
        let store = StateStore::new(Box::new(backend));

        let state_a = WorkflowState::new("alpha");
        let state_b = WorkflowState::new("beta");

        store
            .save_workflow_state("alpha", &state_a)
            .await
            .expect("save failed");
        store
            .save_workflow_state("beta", &state_b)
            .await
            .expect("save failed");

        let workflows = store.list_workflows().await.expect("list failed");
        assert_eq!(workflows.len(), 2);
        assert!(workflows.contains(&"alpha".to_string()));
        assert!(workflows.contains(&"beta".to_string()));

        // Delete one
        store
            .delete_workflow_state("alpha")
            .await
            .expect("delete failed");

        let workflows = store.list_workflows().await.expect("list failed");
        assert_eq!(workflows.len(), 1);
        assert!(workflows.contains(&"beta".to_string()));
    }

    #[tokio::test]
    async fn test_state_store_snapshot_save_load() {
        let tmp = TempDir::new().expect("Failed to create temp dir");
        let backend = JsonFileBackend::new(tmp.path().to_path_buf());
        let store = StateStore::new(Box::new(backend));

        let mut ws = WorkflowState::new("snap-test");
        ws.record_event();
        ws.record_event();
        ws.record_action();

        let snap = Snapshot::from_workflow_state("snap-test", &ws);
        store
            .save_snapshot(&snap)
            .await
            .expect("save snapshot failed");

        let loaded = store
            .load_latest_snapshot("snap-test")
            .await
            .expect("load snapshot failed")
            .expect("snapshot should exist");

        assert_eq!(loaded.workflow_name, "snap-test");
        assert_eq!(loaded.events_processed, 2);
        assert_eq!(loaded.actions_executed, 1);
    }
}
