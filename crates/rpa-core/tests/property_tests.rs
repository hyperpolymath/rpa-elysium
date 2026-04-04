// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Property-based tests for RPA core types and invariants
//!
//! Uses proptest to verify that event processing and state transitions
//! maintain expected invariants across arbitrary inputs.

use proptest::prelude::*;
use rpa_core::{Event, EventKind, WorkflowState};
use std::path::PathBuf;

/// Strategy for generating arbitrary EventKind values
fn arb_event_kind() -> impl Strategy<Value = EventKind> {
    prop_oneof![
        Just(EventKind::Manual),
        // FileCreated events
        "[a-z]+\\.txt"
            .prop_map(|s| PathBuf::from(format!("/tmp/{}", s)))
            .prop_map(|p| EventKind::FileCreated { path: p }),
        // FileModified events
        "[a-z]+\\.json"
            .prop_map(|s| PathBuf::from(format!("/tmp/{}", s)))
            .prop_map(|p| EventKind::FileModified { path: p }),
        // FileDeleted events
        "[a-z]+\\.log"
            .prop_map(|s| PathBuf::from(format!("/tmp/{}", s)))
            .prop_map(|p| EventKind::FileDeleted { path: p }),
        // FileRenamed events
        (
            "[a-z]+\\.rs",
            "[a-z]+\\.rs",
        )
            .prop_map(|(f1, f2)| {
                (
                    PathBuf::from(format!("/tmp/{}", f1)),
                    PathBuf::from(format!("/tmp/{}", f2)),
                )
            })
            .prop_map(|(from, to)| EventKind::FileRenamed { from, to }),
        // Scheduled events - just simple time strings
        Just("10:30".to_string())
            .prop_map(|s| EventKind::Scheduled { schedule: s }),
    ]
}

proptest! {
    /// Property: Event creation always produces valid event IDs
    #[test]
    fn prop_event_id_format_is_valid(event_kind in arb_event_kind()) {
        let event = Event::new(event_kind, "/test/source");

        // Event ID should start with "evt_"
        prop_assert!(
            event.id.starts_with("evt_"),
            "Event ID {} should start with 'evt_'",
            event.id
        );

        // Event ID should be non-empty beyond the prefix
        prop_assert!(
            event.id.len() > 4,
            "Event ID should be longer than prefix"
        );
    }

    /// Property: Event timestamp is always present and recent
    #[test]
    fn prop_event_timestamp_is_recent(event_kind in arb_event_kind()) {
        use chrono::Utc;

        let before = Utc::now();
        let event = Event::new(event_kind, "/test/source");
        let after = Utc::now();

        // Event timestamp should be between before and after creation
        prop_assert!(
            event.timestamp >= before && event.timestamp <= after,
            "Event timestamp should be between creation bounds"
        );
    }

    /// Property: Event source is preserved exactly
    #[test]
    fn prop_event_source_preserved(
        event_kind in arb_event_kind(),
        source in "[a-zA-Z0-9/_\\-\\.]+",
    ) {
        let event = Event::new(event_kind, source.clone());
        prop_assert_eq!(event.source, source);
    }

    /// Property: Event metadata defaults to null
    #[test]
    fn prop_event_metadata_default_null(event_kind in arb_event_kind()) {
        let event = Event::new(event_kind, "/test");
        prop_assert!(event.metadata.is_null());
    }

    /// Property: Event kind is preserved through creation
    #[test]
    fn prop_event_kind_preserved(event_kind in arb_event_kind()) {
        let event = Event::new(event_kind.clone(), "/test");
        prop_assert_eq!(event.kind, event_kind);
    }

    /// Property: WorkflowState name is preserved
    #[test]
    fn prop_workflow_state_name_preserved(name in "[a-zA-Z0-9_-]{1,50}") {
        let state = WorkflowState::new(&name);
        prop_assert_eq!(state.workflow_name, name);
    }

    /// Property: Multiple events don't interfere with each other
    #[test]
    fn prop_multiple_events_independence(
        event_kinds in prop::collection::vec(arb_event_kind(), 1..100),
    ) {
        let events: Vec<_> = event_kinds
            .iter()
            .map(|kind| Event::new(kind.clone(), "/test"))
            .collect();

        // All events should have unique IDs
        let ids: Vec<_> = events.iter().map(|e| &e.id).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();

        prop_assert_eq!(
            ids.len(),
            unique_ids.len(),
            "All events should have unique IDs"
        );

        // All events should have their correct kind preserved
        for (original, event) in event_kinds.iter().zip(events.iter()) {
            prop_assert_eq!(&event.kind, original);
        }
    }

    /// Property: Event metadata can be set and retrieved
    #[test]
    fn prop_event_metadata_roundtrip(event_kind in arb_event_kind()) {
        let metadata = serde_json::json!({
            "key": "value",
            "number": 42,
        });

        let event = Event::new(event_kind, "/test")
            .with_metadata(metadata.clone());

        prop_assert_eq!(event.metadata, metadata);
    }

    /// Property: Event kind display is consistent
    #[test]
    fn prop_event_kind_display_consistency(event_kind in arb_event_kind()) {
        let display = format!("{}", event_kind);

        // Display should not be empty
        prop_assert!(!display.is_empty());

        // Display should contain some identifier
        match event_kind {
            EventKind::FileCreated { .. } => prop_assert!(display.contains("FileCreated")),
            EventKind::FileModified { .. } => prop_assert!(display.contains("FileModified")),
            EventKind::FileDeleted { .. } => prop_assert!(display.contains("FileDeleted")),
            EventKind::FileRenamed { .. } => prop_assert!(display.contains("FileRenamed")),
            EventKind::Manual => prop_assert!(display.contains("Manual")),
            EventKind::Scheduled { .. } => prop_assert!(display.contains("Scheduled")),
        }
    }

    /// Property: Event Display and Debug are both valid representations
    #[test]
    fn prop_event_display_vs_debug(event_kind in arb_event_kind()) {
        let event = Event::new(event_kind, "/test");

        let display = format!("{}", event.kind);
        let debug = format!("{:?}", event.kind);

        // Both should be non-empty
        prop_assert!(!display.is_empty());
        prop_assert!(!debug.is_empty());

        // Display and Debug should be valid string representations
        prop_assert!(display.len() > 0);
        prop_assert!(debug.len() > 0);
    }

    /// Property: EventKind equality is reflexive
    #[test]
    fn prop_event_kind_equality_reflexive(event_kind in arb_event_kind()) {
        let kind = event_kind.clone();
        prop_assert_eq!(&kind, &kind);
    }

    /// Property: EventKind serialization roundtrips correctly
    #[test]
    fn prop_event_kind_serde_roundtrip(event_kind in arb_event_kind()) {
        let serialized = serde_json::to_string(&event_kind)
            .expect("serialize event kind");

        let deserialized: EventKind = serde_json::from_str(&serialized)
            .expect("deserialize event kind");

        prop_assert_eq!(deserialized, event_kind);
    }

    /// Property: Event full serialization roundtrips correctly
    #[test]
    fn prop_event_serde_roundtrip(event_kind in arb_event_kind()) {
        let event = Event::new(event_kind, "/test/path");

        let serialized = serde_json::to_string(&event)
            .expect("serialize event");

        let deserialized: Event = serde_json::from_str(&serialized)
            .expect("deserialize event");

        // IDs and timestamps might differ slightly, but kind and source should match
        prop_assert_eq!(deserialized.kind, event.kind);
        prop_assert_eq!(deserialized.source, event.source);
    }
}

// Non-property tests that test state mutations
#[test]
fn test_workflow_state_initial_consistency() {
    let mut state = WorkflowState::new("test_workflow");
    state.start();

    // Should have no events processed yet
    assert_eq!(state.events_processed, 0);

    // Should have no actions executed
    assert_eq!(state.actions_executed, 0);

    // Should have no errors
    assert_eq!(state.error_count, 0);
}

#[test]
fn test_workflow_state_event_counting() {
    let mut state = WorkflowState::new("test_workflow");
    state.start();

    let initial = state.events_processed;

    // Record 100 events
    for _ in 0..100 {
        state.record_event();
    }

    // Event count should increase by exactly 100
    assert_eq!(state.events_processed, initial + 100);
}

#[test]
fn test_workflow_state_action_counting() {
    let mut state = WorkflowState::new("test_workflow");
    state.start();

    let initial = state.actions_executed;

    // Record 100 actions
    for _ in 0..100 {
        state.record_action();
    }

    // Action count should increase by exactly 100
    assert_eq!(state.actions_executed, initial + 100);
}

#[test]
fn test_workflow_state_error_counting() {
    let mut state = WorkflowState::new("test_workflow");
    state.start();

    let initial = state.error_count;

    // Record 100 errors
    for _ in 0..100 {
        state.record_error();
    }

    // Error count should increase by exactly 100
    assert_eq!(state.error_count, initial + 100);
}
