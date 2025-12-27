// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>

//! Event types for workflow triggers

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents an event that can trigger workflow actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for this event
    pub id: String,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// The type of event
    pub kind: EventKind,
    /// Source of the event (e.g., path, URL, etc.)
    pub source: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Event {
    /// Create a new event
    pub fn new(kind: EventKind, source: impl Into<String>) -> Self {
        Self {
            id: generate_event_id(),
            timestamp: Utc::now(),
            kind,
            source: source.into(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Add metadata to the event
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Types of events that can occur
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    /// A file was created
    FileCreated { path: PathBuf },
    /// A file was modified
    FileModified { path: PathBuf },
    /// A file was deleted
    FileDeleted { path: PathBuf },
    /// A file was renamed/moved
    FileRenamed { from: PathBuf, to: PathBuf },
    /// Manual trigger
    Manual,
    /// Scheduled trigger
    Scheduled { schedule: String },
}

fn generate_event_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("evt_{:x}_{:x}", now.as_secs(), now.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new(
            EventKind::FileCreated {
                path: PathBuf::from("/tmp/test.txt"),
            },
            "/tmp",
        );
        assert!(event.id.starts_with("evt_"));
        assert_eq!(event.source, "/tmp");
    }
}
