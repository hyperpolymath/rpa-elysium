-- SPDX-License-Identifier: PMPL-1.0-or-later
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
-- Foreign.idr — FFI declarations for the Zig implementation layer
--
-- Declares the foreign functions that the Zig FFI must implement,
-- along with their type signatures and calling conventions.

module RpaElysium.Abi.Foreign

import RpaElysium.Abi.Types
import RpaElysium.Abi.Layout

%default total

||| Opaque handle for a workflow instance on the FFI side
public export
data WorkflowHandle : Type where
  MkWorkflowHandle : (ptr : AnyPtr) -> WorkflowHandle

||| Opaque handle for an event on the FFI side
public export
data EventHandle : Type where
  MkEventHandle : (ptr : AnyPtr) -> EventHandle

||| FFI calling convention marker
public export
data CallingConvention : Type where
  CDecl   : CallingConvention
  StdCall : CallingConvention

-- ============================================================
-- FFI function declarations
-- These document the C-compatible interface that ffi/zig/
-- must implement. Actual %foreign pragmas would go here
-- once the Zig implementation is complete.
-- ============================================================

||| Create a new workflow instance
||| C signature: rpa_workflow_create(const char* name) -> WorkflowHandle*
||| Returns: opaque handle or NULL on error
export
workflowCreateSig : String -> IO (Maybe WorkflowHandle)
-- %foreign "C:rpa_workflow_create,librpa_ffi"
workflowCreateSig _ = pure Nothing  -- Stub until Zig FFI is built

||| Destroy a workflow instance and free resources
||| C signature: rpa_workflow_destroy(WorkflowHandle* handle) -> void
export
workflowDestroySig : WorkflowHandle -> IO ()
-- %foreign "C:rpa_workflow_destroy,librpa_ffi"
workflowDestroySig _ = pure ()  -- Stub

||| Get the current workflow status
||| C signature: rpa_workflow_status(WorkflowHandle* handle) -> uint8_t
export
workflowStatusSig : WorkflowHandle -> IO Bits8
-- %foreign "C:rpa_workflow_status,librpa_ffi"
workflowStatusSig _ = pure 0  -- Stub (0 = Idle)

||| Process an event through the workflow
||| C signature: rpa_event_process(WorkflowHandle* handle, EventHandle* event) -> int32_t
||| Returns: 0 on success, error code on failure
export
eventProcessSig : WorkflowHandle -> EventHandle -> IO Int32
-- %foreign "C:rpa_event_process,librpa_ffi"
eventProcessSig _ _ = pure 0  -- Stub

||| Get the last error message
||| C signature: rpa_last_error(char* buf, size_t buf_len) -> int32_t
||| Returns: number of bytes written, or -1 if no error
export
lastErrorSig : IO Int32
-- %foreign "C:rpa_last_error,librpa_ffi"
lastErrorSig = pure (-1)  -- Stub (no error)

||| Get library version string
||| C signature: rpa_version() -> const char*
export
versionSig : IO String
-- %foreign "C:rpa_version,librpa_ffi"
versionSig = pure "0.1.0"  -- Stub
