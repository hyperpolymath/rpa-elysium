<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-03-17 -->

# RPA Elysium — Project Topology

## System Architecture

```
                        ┌─────────────────────────────────────────┐
                        │              BUSINESS USER              │
                        │        (Management Console / UI)        │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │           MANAGEMENT CONSOLE            │
                        │  ┌───────────┐  ┌───────────────────┐  │
                        │  │ Workflow  │  │  Monitoring &     │  │
                        │  │ Designer  │  │  Analytics        │  │
                        │  └─────┬─────┘  └────────┬──────────┘  │
                        └────────│─────────────────│──────────────┘
                                 │                 │
                                 ▼                 ▼
                        ┌─────────────────────────────────────────┐
                        │           BOT FRAMEWORK (RUST)          │
                        │    (Scheduling, State, Resource Mgmt)   │
                        └──────────┬───────────────────┬──────────┘
                                   │                   │
                                   ▼                   ▼
                        ┌───────────────────────┐  ┌────────────────────────────────┐
                        │ AUTOMATION MODULES    │  │ AI & INTELLIGENCE              │
                        │ - Web / Desktop       │  │ - OCR / Form Recognition       │
                        │ - API Integration     │  │ - NLP / Pattern Learning       │
                        │ - Doc Processing      │  │ - Rust + WASM Engine           │
                        └──────────┬────────────┘  └──────────┬─────────────────────┘
                                   │                          │
                                   └────────────┬─────────────┘
                                                ▼
                        ┌─────────────────────────────────────────┐
                        │           BACKEND SERVICES              │
                        │      (Gleam / BEAM, Job Queue)          │
                        └─────────────────────────────────────────┘

                        ┌─────────────────────────────────────────┐
                        │          ABI / FFI LAYER                │
                        │  Idris2 ABI Defs     Zig C-FFI Impl    │
                        │  src/abi/            ffi/zig/           │
                        └─────────────────────────────────────────┘

                        ┌─────────────────────────────────────────┐
                        │          REPO INFRASTRUCTURE            │
                        │  Justfile Automation  .machine_readable/  │
                        │  Multi-Forge Hub      0-AI-MANIFEST.a2ml  │
                        └─────────────────────────────────────────┘
```

## Completion Dashboard

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
RUST WORKSPACE
  rpa-core                          ██████████ 100%    Types, traits, error handling
  rpa-config                        ██████████ 100%    Configuration loading, Nickel integration
  rpa-events                        ██████████ 100%    Async pub/sub event bus, lifecycle hooks
  rpa-plugin                        ████████░░  85%    WASM sandbox, host, permissions (host not wired — Phase 2)
  rpa-resources                     ██████████ 100%    Resource allocation, semaphore-based pooling
  rpa-scheduler                     ██████████ 100%    Cron-like task scheduling
  rpa-state                         ██████████ 100%    Bot state management, persistence
  rpa-fs-workflow                   █████████░  95%    CLI, watcher, 6 action types

PLATFORM (PLANNED)
  Management Console (ReScript)     █░░░░░░░░░  10%    Initial design stubs
  Automation Modules                █░░░░░░░░░  10%    Pending implementation
  AI Intelligence Engine            █░░░░░░░░░  10%    WASM component stubs
  Backend Services (Gleam)          █░░░░░░░░░  10%    Scaffold only

ABI / FFI / PROVEN
  Idris2 ABI (core types)          ███░░░░░░░  30%    Types + Layout + Foreign
  proven-fsm bindings              ██████████ 100%    ProvenFSM.idr complete
  proven-queueconn bindings        ██████████ 100%    ProvenQueue.idr complete
  Ephapax linear types             ██████████ 100%    LinearDispatch.eph complete
  Zig FFI implementation           █░░░░░░░░░  10%    Build scaffold only

PANLL PANELS
  fs-workflow panel                ██████████ 100%    Status, rules, timeline, FSM
  plugin-status panel              ██████████ 100%    Plugins, sandbox, logs

INFRASTRUCTURE
  CI/CD Pipelines (17+)            ██████████ 100%    Forge sync stable
  Governance & Standards            ██████████ 100%    RSR Gold scaffolding verified
  .machine_readable/                ██████████ 100%    STATE tracking active

REPO INFRASTRUCTURE
  Justfile Automation               ██████████ 100%    Standard build tasks
  0-AI-MANIFEST.a2ml                ██████████ 100%    AI entry point verified
  Language Policy (CCCP)            ██████████ 100%    RSR stack verified

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            █████░░░░░  ~55%   Phase 1 Complete, Phase 2 Planned
```

## Key Dependencies

```
RSR Standards ───► Infrastructure ───► Core Framework ───► Automation
     │                 │                   │                 │
     ▼                 ▼                   ▼                 ▼
CCCP Policy ───► CI Workflows ─────► Intelligence ─────► Management
```

## Update Protocol

This file is maintained by both humans and AI agents. When updating:

1. **After completing a component**: Change its bar and percentage
2. **After adding a component**: Add a new row in the appropriate section
3. **After architectural changes**: Update the ASCII diagram
4. **Date**: Update the `Last updated` comment at the top of this file

Progress bars use: `█` (filled) and `░` (empty), 10 characters wide.
Percentages: 0%, 10%, 20%, ... 100% (in 10% increments).
