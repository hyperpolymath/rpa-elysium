<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
# ABI/FFI Architecture — RPA Elysium

## Overview

RPA Elysium uses the **hyperpolymath universal ABI/FFI standard**:

| Layer | Language | Purpose | Location |
|-------|----------|---------|----------|
| **ABI** | Idris2 | Interface definitions with formal proofs | `src/abi/` |
| **FFI** | Zig | C-compatible implementation | `ffi/zig/` |
| **Headers** | C (generated) | Bridge between ABI and FFI | `generated/abi/` |

## Why This Architecture

### Idris2 for ABI

- **Dependent types** prove interface correctness at compile-time
- Formal verification of memory layout and alignment
- Platform-specific ABIs with compile-time selection
- Provable backward compatibility between versions
- Type-level guarantees impossible in C/Zig/Rust alone

### Zig for FFI

- Native C ABI compatibility without overhead
- Memory-safe by default with no hidden allocations
- Cross-compilation to 50+ targets built-in
- No runtime dependencies
- Zero-cost abstractions matching C performance

## Directory Structure

```
src/abi/
  Types.idr          — Core RPA types (Event, Action, Workflow, Error)
  Layout.idr         — Memory layout proofs and alignment guarantees
  Foreign.idr        — FFI function declarations and calling conventions
  ProvenFSM.idr      — proven-fsm bindings (workflow state machine)
  ProvenQueue.idr    — proven-queueconn bindings (event queue interface)
  LinearDispatch.eph — Ephapax linear types (single-use event dispatch)

ffi/zig/
  build.zig          — Zig build configuration (shared + static lib)
  src/main.zig       — C-compatible FFI implementation stubs

generated/abi/       — Auto-generated C headers (not yet implemented)
```

## Type Mapping

| Idris2 Type | Zig Type | C Type | Notes |
|-------------|----------|--------|-------|
| `EventKind` | `EventKind` (enum u8) | `uint8_t` | Tagged union, 6 variants |
| `Timestamp` | `Timestamp` (extern struct) | `struct rpa_timestamp` | 16 bytes, 8-aligned |
| `WorkflowStatus` | `WorkflowStatus` (enum u8) | `uint8_t` | 5 variants |
| `ActionResult` | `ActionResult` (extern struct) | `struct rpa_action_result` | 16 bytes, 8-aligned |
| `RpaError` | `ErrorCode` (enum u32) | `uint32_t` | 6 error codes |

## Building

```bash
# Build the Zig FFI library
cd ffi/zig && zig build

# Run FFI tests
cd ffi/zig && zig build test

# Build the Rust workspace (does not depend on FFI yet)
cargo build --workspace
```

## Proven-Servers Integration

RPA Elysium integrates with the **proven-servers** ecosystem for formally
verified state machines and message queue interfaces:

### proven-fsm (ProvenFSM.idr)

Maps proven-fsm's linear finite state machine types onto RPA Elysium's
workflow lifecycle:

| proven-fsm `MachineState` | RPA Elysium `WorkflowStatus` | Tag |
|---------------------------|------------------------------|-----|
| `Initial` | `Idle` | 0 |
| `Running` | `Running` | 1 |
| `Terminal` | `Stopped` | 2 |
| `Faulted` | `Error` | 3 |

Additional types imported:
- `TransitionResult` (Accepted/Rejected/Deferred) — workflow state change outcomes
- `EventDisposition` (Consumed/Ignored/Queued/Dropped) — event processing results
- `ValidMachineTransition` — proof-carrying type for legal state transitions

### proven-queueconn (ProvenQueue.idr)

Maps proven-queueconn's queue connector types onto RPA Elysium's event
consumption layer (events routed from the Hybrid Automation Router):

- `QueueState` — subscription lifecycle (Disconnected/Connected/Consuming/Producing/Failed)
- `MessageState` — individual event lifecycle (Pending/Delivered/Acknowledged/Rejected/DeadLettered/Expired)
- `DeliveryGuarantee` — per-workflow delivery semantics (AtMostOnce/AtLeastOnce/ExactlyOnce)
- `QueueOp` — queue operations (Publish/Subscribe/Acknowledge/Reject/Peek/Purge)
- `ValidQueueTransition` — proof-carrying type for legal subscription state transitions

### Ephapax Linear Types (LinearDispatch.eph)

Defines linear types that enforce single-use semantics at compile time:

| Type | Guarantee |
|------|-----------|
| `RoutedEvent linear` | Event consumed exactly once — no duplication, no silent drop |
| `WorkflowTransition linear` | Transition executed exactly once — no re-execution |
| `QueueLease linear` | Subscription handle explicitly released — no resource leak |

Uses `let!` bindings to enforce that linear values are consumed exactly once
within their scope.  The `withLease` pattern provides automatic lease
lifecycle management.

## Current Status

- [x] Idris2 ABI type definitions (scaffold)
- [x] Idris2 layout proofs (scaffold)
- [x] Idris2 FFI declarations (scaffold)
- [x] Zig build configuration
- [x] Zig FFI stubs with tests
- [x] proven-fsm bindings (ProvenFSM.idr)
- [x] proven-queueconn bindings (ProvenQueue.idr)
- [x] Ephapax linear types (LinearDispatch.eph)
- [ ] C header generation from Idris2
- [ ] Wire FFI into Rust via `extern "C"` bindings
- [ ] Integration tests (Rust <-> Zig via C ABI)
- [ ] Platform-specific ABI selection (Linux/macOS/Windows)

## References

- [hyperpolymath ABI/FFI standard](https://github.com/hyperpolymath/rsr-template-repo/blob/main/ABI-FFI-README.md)
- [Idris2 FFI documentation](https://idris2.readthedocs.io/en/latest/ffi/index.html)
- [Zig C interop guide](https://ziglang.org/documentation/master/#C-Type-Coercions)
