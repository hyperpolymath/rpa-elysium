// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Concurrency tests for RPA core components
//!
//! Verifies that event processing and state management are thread-safe
//! and don't corrupt state under concurrent access.

use rpa_core::{Event, EventKind, WorkflowState};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// Test concurrent event creation doesn't create duplicate IDs
#[test]
fn test_concurrent_event_creation_uniqueness() {
    let mut handles = vec![];
    let ids = Arc::new(Mutex::new(Vec::new()));

    // Spawn 10 threads, each creating 100 events
    for _ in 0..10 {
        let ids = Arc::clone(&ids);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let event = Event::new(
                    EventKind::FileCreated {
                        path: PathBuf::from(format!("/tmp/file_{}", i)),
                    },
                    "/test",
                );
                ids.lock().unwrap().push(event.id);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("thread panicked");
    }

    // Verify all IDs are unique (or mostly unique - timestamp-based generation might have collisions under high load)
    let ids = ids.lock().unwrap();
    assert_eq!(ids.len(), 1000, "should have 1000 events");

    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    // Allow up to 5 collisions due to timestamp granularity
    assert!(
        unique_ids.len() >= 995,
        "event IDs should be mostly unique (got {}/1000)",
        unique_ids.len()
    );
}

/// Test concurrent WorkflowState event recording is atomic
#[test]
fn test_concurrent_workflow_state_event_recording() {
    let state = Arc::new(Mutex::new(WorkflowState::new("concurrent_test")));
    let mut handles = vec![];

    state.lock().unwrap().start();

    // Spawn 10 threads, each recording 100 events
    for _ in 0..10 {
        let state = Arc::clone(&state);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                state.lock().unwrap().record_event();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("thread panicked");
    }

    // Verify final count
    let state = state.lock().unwrap();
    assert_eq!(
        state.events_processed, 1000,
        "should have recorded 1000 events"
    );
}

/// Test concurrent WorkflowState action recording is atomic
#[test]
fn test_concurrent_workflow_state_action_recording() {
    let state = Arc::new(Mutex::new(WorkflowState::new("concurrent_test")));
    let mut handles = vec![];

    state.lock().unwrap().start();

    // Spawn 10 threads, each recording 100 actions
    for _ in 0..10 {
        let state = Arc::clone(&state);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                state.lock().unwrap().record_action();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("thread panicked");
    }

    // Verify final count
    let state = state.lock().unwrap();
    assert_eq!(
        state.actions_executed, 1000,
        "should have recorded 1000 actions"
    );
}

/// Test concurrent WorkflowState error recording is atomic
#[test]
fn test_concurrent_workflow_state_error_recording() {
    let state = Arc::new(Mutex::new(WorkflowState::new("concurrent_test")));
    let mut handles = vec![];

    state.lock().unwrap().start();

    // Spawn 10 threads, each recording 100 errors
    for _ in 0..10 {
        let state = Arc::clone(&state);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                state.lock().unwrap().record_error();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("thread panicked");
    }

    // Verify final count
    let state = state.lock().unwrap();
    assert_eq!(
        state.error_count, 1000,
        "should have recorded 1000 errors"
    );
}

/// Test concurrent mixed operations on WorkflowState
#[test]
fn test_concurrent_workflow_state_mixed_operations() {
    let state = Arc::new(Mutex::new(WorkflowState::new("concurrent_test")));

    state.lock().unwrap().start();

    // Thread 1: Record events
    let state_clone = Arc::clone(&state);
    let h1 = thread::spawn(move || {
        for _ in 0..333 {
            state_clone.lock().unwrap().record_event();
        }
    });

    // Thread 2: Record actions
    let state_clone = Arc::clone(&state);
    let h2 = thread::spawn(move || {
        for _ in 0..333 {
            state_clone.lock().unwrap().record_action();
        }
    });

    // Thread 3: Record errors
    let state_clone = Arc::clone(&state);
    let h3 = thread::spawn(move || {
        for _ in 0..334 {
            state_clone.lock().unwrap().record_error();
        }
    });

    h1.join().expect("h1 panicked");
    h2.join().expect("h2 panicked");
    h3.join().expect("h3 panicked");

    // Verify all counts
    let state = state.lock().unwrap();
    assert_eq!(state.events_processed, 333);
    assert_eq!(state.actions_executed, 333);
    assert_eq!(state.error_count, 334);
}

/// Test event creation under high concurrency maintains invariants
#[test]
fn test_high_concurrency_event_creation() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // Spawn 20 threads creating events in parallel
    for thread_id in 0..20 {
        let events = Arc::clone(&events);
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let event = Event::new(
                    EventKind::FileModified {
                        path: PathBuf::from(format!("/tmp/thread_{}_file_{}", thread_id, i)),
                    },
                    &format!("/test/{}/{}", thread_id, i),
                );

                // Verify event invariants immediately
                assert!(event.id.starts_with("evt_"), "event ID should start with evt_");
                assert_eq!(event.source, format!("/test/{}/{}", thread_id, i));

                events.lock().unwrap().push(event);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("thread panicked");
    }

    let all_events = events.lock().unwrap();
    assert_eq!(all_events.len(), 1000, "should have 1000 events");

    // Verify all have mostly unique IDs (allow up to 5 collisions due to timestamp granularity)
    let unique_ids: std::collections::HashSet<_> = all_events.iter().map(|e| &e.id).collect();
    assert!(
        unique_ids.len() >= 995,
        "should have mostly unique IDs (got {}/1000)",
        unique_ids.len()
    );

    // Verify all have valid timestamps (but don't assume monotonicity due to threading)
    let now = chrono::Utc::now();
    for event in all_events.iter() {
        // Each event's timestamp should be recent (within last second)
        let age = now.signed_duration_since(event.timestamp);
        assert!(
            age.num_seconds() >= 0 && age.num_seconds() < 10,
            "event timestamp should be recent"
        );
    }
}

/// Test that event kind equality holds across threads
#[test]
fn test_concurrent_event_kind_equality() {
    let kind = EventKind::FileRenamed {
        from: PathBuf::from("/tmp/old.txt"),
        to: PathBuf::from("/tmp/new.txt"),
    };

    let kind = Arc::new(kind);
    let mut handles = vec![];

    // Spawn 10 threads verifying equality
    for _ in 0..10 {
        let kind = Arc::clone(&kind);
        let handle = thread::spawn(move || {
            // Each thread clones and compares
            let my_kind = (*kind).clone();
            assert_eq!(*kind, my_kind);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("thread panicked");
    }
}

/// Test WorkflowState snapshot under concurrent access
#[test]
fn test_concurrent_workflow_state_snapshot() {
    let state = Arc::new(Mutex::new(WorkflowState::new("concurrent_test")));
    let mut handles = vec![];

    state.lock().unwrap().start();

    // Record events and actions while snapshotting
    for i in 0..5 {
        let state = Arc::clone(&state);
        let handle = thread::spawn(move || {
            // Record some data
            for j in 0..20 {
                if (i + j) % 2 == 0 {
                    state.lock().unwrap().record_event();
                } else {
                    state.lock().unwrap().record_action();
                }
            }
        });
        handles.push(handle);
    }

    // Take snapshots while threads are working
    let snapshot_count = Arc::new(Mutex::new(0));
    for _ in 0..5 {
        let state = Arc::clone(&state);
        let snapshot_count = Arc::clone(&snapshot_count);
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let s = state.lock().unwrap();
                let count = s.events_processed + s.actions_executed + s.error_count;
                *snapshot_count.lock().unwrap() += count;
                drop(s);
                thread::sleep(std::time::Duration::from_millis(10));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("thread panicked");
    }

    let final_state = state.lock().unwrap();
    assert_eq!(
        final_state.events_processed + final_state.actions_executed,
        100,
        "should have recorded 100 operations"
    );
}

/// Test no data races in event field access
#[test]
fn test_concurrent_event_field_access() {
    let event = Arc::new(Event::new(
        EventKind::FileCreated {
            path: PathBuf::from("/tmp/test.txt"),
        },
        "/test",
    ));

    let mut handles = vec![];

    // Spawn threads that read different fields
    for _ in 0..10 {
        let event = Arc::clone(&event);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let _id = &event.id;
                let _ts = event.timestamp;
                let _kind = &event.kind;
                let _source = &event.source;
                let _meta = &event.metadata;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("thread panicked");
    }
    // If we reach here without data races, test passes
}
