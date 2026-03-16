-- SPDX-License-Identifier: PMPL-1.0-or-later
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
-- Types.idr — Core RPA type definitions for the ABI layer
--
-- These types mirror the Rust rpa-core types (Event, Action, Workflow, Error)
-- and provide formal proofs of their properties via dependent types.

module RpaElysium.Abi.Types

import Data.Vect
import Data.String

%default total

||| Maximum length of an event ID string (64 bytes)
public export
EventIdMaxLen : Nat
EventIdMaxLen = 64

||| Maximum length of a source string (256 bytes)
public export
SourceMaxLen : Nat
SourceMaxLen = 256

||| Bounded string with compile-time length proof
public export
record BoundedString (maxLen : Nat) where
  constructor MkBoundedString
  value : String
  0 lengthProof : LTE (length value) maxLen

||| Event kinds matching Rust EventKind enum
public export
data EventKind : Type where
  FileCreated  : (path : String) -> EventKind
  FileModified : (path : String) -> EventKind
  FileDeleted  : (path : String) -> EventKind
  FileRenamed  : (from : String) -> (to : String) -> EventKind
  Manual       : EventKind
  Scheduled    : (schedule : String) -> EventKind

||| Tag values for C ABI representation of EventKind
public export
eventKindTag : EventKind -> Bits8
eventKindTag (FileCreated _)    = 0
eventKindTag (FileModified _)   = 1
eventKindTag (FileDeleted _)    = 2
eventKindTag (FileRenamed _ _)  = 3
eventKindTag Manual             = 4
eventKindTag (Scheduled _)      = 5

||| Proof that event kind tags are within valid range
public export
eventKindTagValid : (ek : EventKind) -> LTE (cast (eventKindTag ek)) 5
eventKindTagValid (FileCreated _)    = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagValid (FileModified _)   = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagValid (FileDeleted _)    = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagValid (FileRenamed _ _)  = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagValid Manual             = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagValid (Scheduled _)      = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))

||| Timestamp as Unix epoch seconds + nanoseconds
public export
record Timestamp where
  constructor MkTimestamp
  seconds : Int64
  nanos   : Bits32

||| Core Event type matching Rust Event struct
public export
record Event where
  constructor MkEvent
  id        : String
  timestamp : Timestamp
  kind      : EventKind
  source    : String

||| Action result matching Rust ActionResult struct
public export
record ActionResult where
  constructor MkActionResult
  success : Bool
  message : String

||| Workflow status matching Rust WorkflowStatus enum
public export
data WorkflowStatus : Type where
  Idle    : WorkflowStatus
  Running : WorkflowStatus
  Paused  : WorkflowStatus
  Stopped : WorkflowStatus
  Error   : WorkflowStatus

||| Workflow state matching Rust WorkflowState struct
public export
record WorkflowState where
  constructor MkWorkflowState
  workflowName    : String
  status          : WorkflowStatus
  eventsProcessed : Nat
  actionsExecuted : Nat
  errorCount      : Nat

||| Error codes for the FFI boundary
public export
data RpaError : Type where
  ErrIo             : (msg : String) -> RpaError
  ErrConfig         : (msg : String) -> RpaError
  ErrWorkflow       : (msg : String) -> RpaError
  ErrActionFailed   : (action : String) -> (reason : String) -> RpaError
  ErrInvalidPattern : (msg : String) -> RpaError
  ErrWatch          : (msg : String) -> RpaError

||| Numeric error codes for C ABI
public export
errorCode : RpaError -> Bits32
errorCode (ErrIo _)              = 1
errorCode (ErrConfig _)          = 2
errorCode (ErrWorkflow _)        = 3
errorCode (ErrActionFailed _ _)  = 4
errorCode (ErrInvalidPattern _)  = 5
errorCode (ErrWatch _)           = 6
