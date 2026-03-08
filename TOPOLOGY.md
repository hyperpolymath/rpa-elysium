<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-02-19 -->

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
                        │          REPO INFRASTRUCTURE            │
                        │  Justfile Automation  .machine_readable/  │
                        │  Multi-Forge Hub      0-AI-MANIFEST.a2ml  │
                        └─────────────────────────────────────────┘
```

## Completion Dashboard

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
PLATFORM CORE
  Bot Framework (Rust)              █░░░░░░░░░  10%    Architecture stubs
  Management Console (ReScript)     █░░░░░░░░░  10%    Initial UI design stubs
  Automation Modules                █░░░░░░░░░  10%    Pending implementation
  AI Intelligence Engine            █░░░░░░░░░  10%    WASM component stubs

INFRASTRUCTURE
  CI/CD Pipelines (11)              ██████████ 100%    Forge sync stable
  Governance & Standards            ██████████ 100%    RSR Gold scaffolding verified
  .machine_readable/                ██████████ 100%    STATE tracking active

REPO INFRASTRUCTURE
  Justfile Automation               ██████████ 100%    Standard build tasks
  0-AI-MANIFEST.a2ml                ██████████ 100%    AI entry point verified
  Language Policy (CCCP)            ██████████ 100%    RSR stack verified

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            ██░░░░░░░░  ~20%   Specification Phase complete
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
