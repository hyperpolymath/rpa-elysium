-- SPDX-License-Identifier: PMPL-1.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
-- ProvenFSM.idr — Proven-servers FSM bindings for RPA Elysium
--
-- Imports and re-exports proven-fsm types adapted for RPA Elysium's workflow
-- state machine.  The proven-fsm library provides formally verified linear
-- finite state machines; this module maps its vocabulary onto the RPA domain.
--
-- Mapping:
--   proven-fsm MachineState   →  WorkflowStatus (from Types.idr)
--     Initial  = Idle     (tag 0)
--     Running  = Running  (tag 1)
--     Terminal = Stopped  (tag 3)
--     Faulted  = Error    (tag 4)
--
--   TransitionResult           →  used for workflow state changes
--   EventDisposition           →  used when events arrive at the workflow engine
--   ValidMachineTransition     →  defines which workflow state transitions are legal

module RpaElysium.Abi.ProvenFSM

import RpaElysium.Abi.Types

%default total

---------------------------------------------------------------------------
-- TransitionResult — the outcome of attempting a workflow state change.
-- Matches proven-fsm FSM.Types.TransitionResult exactly.
---------------------------------------------------------------------------

||| The result of attempting a workflow state transition.
public export
data TransitionResult : Type where
  ||| The transition was accepted and the workflow state has changed.
  Accepted : TransitionResult    -- proven-fsm tag: 0
  ||| The transition was rejected (invalid from current workflow status).
  Rejected : TransitionResult    -- proven-fsm tag: 1
  ||| The transition is valid but deferred for later execution.
  Deferred : TransitionResult    -- proven-fsm tag: 2

||| C ABI tag values — MUST match proven-fsm encoding.
public export
transitionResultTag : TransitionResult -> Bits8
transitionResultTag Accepted = 0
transitionResultTag Rejected = 1
transitionResultTag Deferred = 2

public export
Show TransitionResult where
  show Accepted = "Accepted"
  show Rejected = "Rejected"
  show Deferred = "Deferred"

---------------------------------------------------------------------------
-- ValidationError — why a workflow state transition was rejected.
-- Matches proven-fsm FSM.Types.ValidationError exactly.
---------------------------------------------------------------------------

||| Reasons a workflow transition can fail validation.
public export
data ValidationError : Type where
  ||| The transition is not valid from the current workflow status.
  InvalidTransition   : ValidationError  -- proven-fsm tag: 0
  ||| A precondition guard was not satisfied.
  PreconditionFailed  : ValidationError  -- proven-fsm tag: 1
  ||| A postcondition check was not satisfied.
  PostconditionFailed : ValidationError  -- proven-fsm tag: 2
  ||| A guard function returned false.
  GuardFailed         : ValidationError  -- proven-fsm tag: 3

||| C ABI tag values — MUST match proven-fsm encoding.
public export
validationErrorTag : ValidationError -> Bits8
validationErrorTag InvalidTransition   = 0
validationErrorTag PreconditionFailed  = 1
validationErrorTag PostconditionFailed = 2
validationErrorTag GuardFailed         = 3

public export
Show ValidationError where
  show InvalidTransition   = "InvalidTransition"
  show PreconditionFailed  = "PreconditionFailed"
  show PostconditionFailed = "PostconditionFailed"
  show GuardFailed         = "GuardFailed"

---------------------------------------------------------------------------
-- EventDisposition — what happened to an event after the workflow engine
-- processed it.
-- Matches proven-fsm FSM.Types.EventDisposition exactly.
---------------------------------------------------------------------------

||| What happened to an event after it was submitted to the workflow engine.
public export
data EventDisposition : Type where
  ||| The event was consumed and triggered a workflow state transition.
  Consumed : EventDisposition  -- proven-fsm tag: 0
  ||| The event was not applicable to the current workflow and was ignored.
  Ignored  : EventDisposition  -- proven-fsm tag: 1
  ||| The event was queued for later processing.
  Queued   : EventDisposition  -- proven-fsm tag: 2
  ||| The event was dropped (e.g., event buffer full).
  Dropped  : EventDisposition  -- proven-fsm tag: 3

||| C ABI tag values — MUST match proven-fsm encoding.
public export
eventDispositionTag : EventDisposition -> Bits8
eventDispositionTag Consumed = 0
eventDispositionTag Ignored  = 1
eventDispositionTag Queued   = 2
eventDispositionTag Dropped  = 3

public export
Show EventDisposition where
  show Consumed = "Consumed"
  show Ignored  = "Ignored"
  show Queued   = "Queued"
  show Dropped  = "Dropped"

---------------------------------------------------------------------------
-- WorkflowStatus mapping — maps proven-fsm MachineState to
-- RpaElysium.Abi.Types.WorkflowStatus.
--
-- proven-fsm MachineState   RPA Elysium WorkflowStatus   Tag
-- ─────────────────────────────────────────────────────────────
-- Initial                   Idle                           0
-- Running                   Running                        1
-- Terminal                  Stopped                        3
-- Faulted                   Error                          4
--
-- Note: WorkflowStatus also has Paused (tag 2) which has no proven-fsm
-- counterpart.  Paused is an RPA-specific concept — the machine is
-- still Running in FSM terms but the workflow engine has suspended
-- event processing.
---------------------------------------------------------------------------

||| Map a proven-fsm MachineState tag to the corresponding WorkflowStatus.
||| Returns Nothing for unrecognised tags.
public export
machineStateToWorkflowStatus : Bits8 -> Maybe WorkflowStatus
machineStateToWorkflowStatus 0 = Just Idle     -- Initial
machineStateToWorkflowStatus 1 = Just Running  -- Running
machineStateToWorkflowStatus 2 = Just Stopped  -- Terminal
machineStateToWorkflowStatus 3 = Just Error    -- Faulted
machineStateToWorkflowStatus _ = Nothing

||| Map a WorkflowStatus back to the proven-fsm MachineState tag.
||| Paused has no FSM counterpart and maps to Running (tag 1).
public export
workflowStatusToMachineState : WorkflowStatus -> Bits8
workflowStatusToMachineState Idle    = 0  -- Initial
workflowStatusToMachineState Running = 1  -- Running
workflowStatusToMachineState Paused  = 1  -- Running (closest FSM state)
workflowStatusToMachineState Stopped = 2  -- Terminal
workflowStatusToMachineState Error   = 3  -- Faulted

---------------------------------------------------------------------------
-- ValidMachineTransition — defines which workflow state transitions are
-- legal.  This is a proof-carrying type: a value of
-- ValidMachineTransition s1 s2 can only be constructed for transitions
-- that the workflow engine permits.
---------------------------------------------------------------------------

||| Proof that a workflow state transition from `from` to `to` is valid.
||| Only constructible for legal transitions.
public export
data ValidMachineTransition : (from : WorkflowStatus) -> (to : WorkflowStatus) -> Type where
  ||| Idle → Running: start the workflow.
  StartWorkflow   : ValidMachineTransition Idle Running
  ||| Running → Paused: suspend event processing.
  PauseWorkflow   : ValidMachineTransition Running Paused
  ||| Paused → Running: resume event processing.
  ResumeWorkflow  : ValidMachineTransition Paused Running
  ||| Running → Stopped: workflow completed normally (terminal).
  StopWorkflow    : ValidMachineTransition Running Stopped
  ||| Running → Error: workflow encountered a fatal error (faulted).
  FaultWorkflow   : ValidMachineTransition Running Error
  ||| Paused → Stopped: stop a paused workflow (terminal).
  StopPaused      : ValidMachineTransition Paused Stopped
  ||| Error → Idle: reset a faulted workflow to try again.
  ResetWorkflow   : ValidMachineTransition Error Idle

||| Human-readable name for a valid transition.
public export
Show (ValidMachineTransition from to) where
  show StartWorkflow  = "StartWorkflow (Idle → Running)"
  show PauseWorkflow  = "PauseWorkflow (Running → Paused)"
  show ResumeWorkflow = "ResumeWorkflow (Paused → Running)"
  show StopWorkflow   = "StopWorkflow (Running → Stopped)"
  show FaultWorkflow  = "FaultWorkflow (Running → Error)"
  show StopPaused     = "StopPaused (Paused → Stopped)"
  show ResetWorkflow  = "ResetWorkflow (Error → Idle)"

||| Execute a validated transition, consuming the proof.
||| Returns the new WorkflowStatus and a TransitionResult.
public export
executeTransition : ValidMachineTransition from to -> (WorkflowStatus, TransitionResult)
executeTransition StartWorkflow  = (Running, Accepted)
executeTransition PauseWorkflow  = (Paused,  Accepted)
executeTransition ResumeWorkflow = (Running, Accepted)
executeTransition StopWorkflow   = (Stopped, Accepted)
executeTransition FaultWorkflow  = (Error,   Accepted)
executeTransition StopPaused     = (Stopped, Accepted)
executeTransition ResetWorkflow  = (Idle,    Accepted)
