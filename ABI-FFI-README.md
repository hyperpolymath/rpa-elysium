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
  Types.idr        — Core RPA types (Event, Action, Workflow, Error)
  Layout.idr       — Memory layout proofs and alignment guarantees
  Foreign.idr      — FFI function declarations and calling conventions

ffi/zig/
  build.zig        — Zig build configuration (shared + static lib)
  src/main.zig     — C-compatible FFI implementation stubs

generated/abi/     — Auto-generated C headers (not yet implemented)
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

## Current Status

- [x] Idris2 ABI type definitions (scaffold)
- [x] Idris2 layout proofs (scaffold)
- [x] Idris2 FFI declarations (scaffold)
- [x] Zig build configuration
- [x] Zig FFI stubs with tests
- [ ] C header generation from Idris2
- [ ] Wire FFI into Rust via `extern "C"` bindings
- [ ] Integration tests (Rust <-> Zig via C ABI)
- [ ] Platform-specific ABI selection (Linux/macOS/Windows)

## References

- [hyperpolymath ABI/FFI standard](https://github.com/hyperpolymath/rsr-template-repo/blob/main/ABI-FFI-README.md)
- [Idris2 FFI documentation](https://idris2.readthedocs.io/en/latest/ffi/index.html)
- [Zig C interop guide](https://ziglang.org/documentation/master/#C-Type-Coercions)
