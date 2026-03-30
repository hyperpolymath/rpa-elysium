# TEST-NEEDS: rpa-elysium

## Current State

| Category | Count | Details |
|----------|-------|---------|
| **Source modules** | 36 | 5 crates (rpa-core, rpa-fs-workflow, rpa-plugin, rpa-config, rpa-events) + 5 Idris2 ABI (incl. ProvenFSM, ProvenQueue) |
| **Unit tests (inline)** | 30 | Scattered across crates: config validate=4, plugin sandbox/permissions/host=8, fs-workflow=5, core event=1, others |
| **Integration tests** | 0 | No dedicated integration test files |
| **E2E tests** | 0 | None |
| **Benchmarks** | 0 | None |
| **Fuzz tests** | 0 | None |

## What's Missing

### P2P Tests (CRITICAL)
- [ ] No tests for workflow execution pipeline (core -> fs-workflow -> plugin -> events)
- [ ] No tests for plugin sandboxing with actual filesystem operations

### E2E Tests (CRITICAL)
- [ ] No end-to-end workflow execution test
- [ ] No test that runs a complete file rename/move/archive cycle
- [ ] No test validating ProvenFSM state machine correctness in practice

### Aspect Tests
- [ ] **Security**: rpa-plugin has sandbox tests but only 3 -- insufficient for security-critical code. No escape tests, no capability leakage tests
- [ ] **Performance**: No tests for workflow execution throughput, watcher overhead
- [ ] **Concurrency**: No tests for concurrent file operations, race conditions in watcher
- [ ] **Error handling**: No tests for permission denied, disk full, broken symlinks, locked files

### Build & Execution
- [ ] No Idris2 ABI compilation test for ProvenFSM/ProvenQueue
- [ ] No cross-crate integration build test

### Benchmarks Needed
- [ ] File operation throughput (copy/move/rename at scale)
- [ ] Watcher event processing latency
- [ ] Plugin loading/sandboxing overhead

### Self-Tests
- [ ] No self-diagnostic mode
- [ ] No watcher healthcheck

## FLAGGED ISSUES
- **30 inline tests across 36 source files** = inadequate coverage
- **RPA framework with 0 E2E tests** -- an RPA tool that's never been tested end-to-end
- **Plugin sandbox has 3 tests** -- security-critical component needs 50+

## Priority: P0 (CRITICAL)
