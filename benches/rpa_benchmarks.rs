// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Criterion benchmarks for RPA performance characteristics
//!
//! Measures:
//! - Event creation throughput
//! - WorkflowState operation latency
//! - Permission checking overhead

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rpa_core::{Event, EventKind, WorkflowState};
use rpa_plugin::permissions::{Permission, PermissionSet};
use rpa_plugin::sandbox::SandboxBuilder;
use std::path::PathBuf;

/// Benchmark event creation
fn bench_event_creation(c: &mut Criterion) {
    c.bench_function("event_creation_file_created", |b| {
        b.iter(|| {
            Event::new(
                black_box(EventKind::FileCreated {
                    path: black_box(PathBuf::from("/tmp/test.txt")),
                }),
                black_box("/test"),
            )
        });
    });

    c.bench_function("event_creation_file_renamed", |b| {
        b.iter(|| {
            Event::new(
                black_box(EventKind::FileRenamed {
                    from: black_box(PathBuf::from("/tmp/old.txt")),
                    to: black_box(PathBuf::from("/tmp/new.txt")),
                }),
                black_box("/test"),
            )
        });
    });

    c.bench_function("event_creation_manual", |b| {
        b.iter(|| Event::new(black_box(EventKind::Manual), black_box("/test")));
    });
}

/// Benchmark WorkflowState operations
fn bench_workflow_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("workflow_state");

    group.bench_function("workflow_state_creation", |b| {
        b.iter(|| WorkflowState::new(black_box("test_workflow")));
    });

    group.bench_function("workflow_state_start_stop", |b| {
        b.iter(|| {
            let mut state = WorkflowState::new("test");
            state.start();
            state.stop();
        });
    });

    group.bench_function("workflow_state_record_event", |b| {
        let mut state = WorkflowState::new("test");
        state.start();
        b.iter(|| {
            state.record_event();
        });
    });

    group.bench_function("workflow_state_record_action", |b| {
        let mut state = WorkflowState::new("test");
        state.start();
        b.iter(|| {
            state.record_action();
        });
    });

    group.bench_function("workflow_state_record_error", |b| {
        let mut state = WorkflowState::new("test");
        state.start();
        b.iter(|| {
            state.record_error(black_box("error"));
        });
    });

    group.finish();
}

/// Benchmark permission checking
fn bench_permission_checking(c: &mut Criterion) {
    let mut group = c.benchmark_group("permissions");

    group.bench_function("permission_check_read_path", |b| {
        let set = PermissionSet::empty()
            .with(Permission::read_path("/tmp/data"));

        b.iter(|| {
            set.check(black_box(&Permission::read_path("/tmp/data/file.txt")))
        });
    });

    group.bench_function("permission_check_denied", |b| {
        let set = PermissionSet::empty()
            .with(Permission::read_path("/tmp/data"));

        b.iter(|| {
            set.check(black_box(&Permission::write_path("/tmp/data")))
        });
    });

    group.bench_function("permission_set_creation_small", |b| {
        b.iter(|| {
            black_box(PermissionSet::empty())
                .with(black_box(Permission::Time))
                .with(black_box(Permission::Random))
        });
    });

    group.bench_function("permission_set_creation_large", |b| {
        b.iter(|| {
            let mut set = PermissionSet::empty();
            for i in 0..10 {
                set.add(Permission::read_path(format!("/tmp/path{}", i)));
            }
            set
        });
    });

    group.bench_function("permission_check_multiple_perms", |b| {
        let set = PermissionSet::new(vec![
            Permission::read_path("/tmp"),
            Permission::write_path("/home"),
            Permission::env("HOME"),
            Permission::Time,
            Permission::Random,
        ]);

        b.iter(|| {
            set.check(black_box(&Permission::read_path("/tmp/file.txt")))
        });
    });

    group.finish();
}

/// Benchmark sandbox creation
fn bench_sandbox_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sandbox");

    group.bench_function("sandbox_builder_minimal", |b| {
        b.iter(|| {
            let _sandbox = SandboxBuilder::new().build().ok();
        });
    });

    group.bench_function("sandbox_builder_with_permissions", |b| {
        b.iter(|| {
            let _sandbox = SandboxBuilder::new()
                .permission(black_box(Permission::read_path("/tmp")))
                .permission(black_box(Permission::write_path("/tmp")))
                .permission(black_box(Permission::Time))
                .build()
                .ok();
        });
    });

    group.bench_function("sandbox_builder_full_config", |b| {
        b.iter(|| {
            let _sandbox = SandboxBuilder::new()
                .memory_limit(32 * 1024 * 1024)
                .timeout(15_000)
                .fuel(50_000_000)
                .permission(Permission::read_path("/tmp"))
                .permission(Permission::write_path("/tmp"))
                .permission(Permission::Time)
                .build()
                .ok();
        });
    });

    group.finish();
}

/// Benchmark Event serialization
fn bench_event_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_serialization");

    let event = Event::new(
        EventKind::FileCreated {
            path: PathBuf::from("/tmp/test.txt"),
        },
        "/test",
    );

    group.bench_function("event_serialize_json", |b| {
        b.iter(|| {
            let _json = serde_json::to_string(black_box(&event)).ok();
        });
    });

    group.bench_function("event_serialize_json_pretty", |b| {
        b.iter(|| {
            let _json = serde_json::to_string_pretty(black_box(&event)).ok();
        });
    });

    let json = serde_json::to_string(&event).unwrap();
    group.bench_function("event_deserialize_json", |b| {
        b.iter(|| {
            let _event: Event = serde_json::from_str(black_box(&json)).ok().unwrap();
        });
    });

    group.finish();
}

/// Benchmark EventKind equality
fn bench_event_kind_equality(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_kind");

    let kind1 = EventKind::FileCreated {
        path: PathBuf::from("/tmp/test.txt"),
    };

    let kind2 = EventKind::FileCreated {
        path: PathBuf::from("/tmp/test.txt"),
    };

    group.bench_function("event_kind_equality_same", |b| {
        b.iter(|| black_box(&kind1) == black_box(&kind2))
    });

    let kind3 = EventKind::FileModified {
        path: PathBuf::from("/tmp/other.txt"),
    };

    group.bench_function("event_kind_equality_different", |b| {
        b.iter(|| black_box(&kind1) == black_box(&kind3))
    });

    group.bench_function("event_kind_display", |b| {
        b.iter(|| {
            let _s = format!("{}", black_box(&kind1));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_event_creation,
    bench_workflow_state,
    bench_permission_checking,
    bench_sandbox_creation,
    bench_event_serialization,
    bench_event_kind_equality
);
criterion_main!(benches);
