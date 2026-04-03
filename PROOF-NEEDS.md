# PROOF-NEEDS.md
<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->

## Current State

- **LOC**: ~7,870
- **Languages**: Rust, Idris2, Zig, Nickel
- **Existing ABI proofs**: `src/abi/*.idr` including domain-specific `ProvenFSM.idr` and `ProvenQueue.idr`
- **Dangerous patterns**: None detected

## What Needs Proving

### ProvenFSM Completeness (src/abi/ProvenFSM.idr)
- Finite state machine for RPA workflow execution — already has Idris2 definitions
- Audit: are all transitions proven valid? Are invalid transitions statically rejected?
- Prove: FSM cannot reach error states from valid initial states

### ProvenQueue Correctness (src/abi/ProvenQueue.idr)
- Workflow action queue — already has Idris2 definitions
- Prove: queue operations preserve ordering (FIFO)
- Prove: no action is lost or duplicated during workflow execution

### Workflow Execution Safety (crates/rpa-core/)
- `action.rs`, `workflow.rs`, `event.rs` — core workflow engine
- Prove: workflow execution follows the FSM specification
- Prove: error handling preserves system state (no partial effects on failure)

### File Operations (crates/rpa-fs-workflow/)
- File move, copy, delete, rename, archive operations
- Prove: file operations are atomic or safely reversible
- Prove: plugin-based operations respect the same safety contracts

### Groove Integration (crates/rpa-events/src/groove.rs)
- Event system integration — prove events are delivered exactly once

## Recommended Prover

- **Idris2** (already in use with ProvenFSM and ProvenQueue — complete the proofs)

## Priority

**HIGH** — RPA system performing automated file operations. Incorrect FSM transitions or lost queue actions could cause data loss. The Idris2 proof infrastructure is already in place — completing it is the highest-ROI work.
