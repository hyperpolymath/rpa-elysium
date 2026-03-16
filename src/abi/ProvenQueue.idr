-- SPDX-License-Identifier: PMPL-1.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
-- ProvenQueue.idr — Proven-servers QueueConn bindings for RPA Elysium
--
-- Imports and re-exports proven-queueconn types adapted for RPA Elysium's
-- event consumption layer.  The Hybrid Automation Router (HAR) routes events
-- to RPA Elysium workflows; this module provides the type-safe queue
-- interface for receiving and processing those routed events.
--
-- Mapping:
--   QueueState        →  subscription lifecycle for receiving routed events
--   MessageState      →  tracking individual automation events through processing
--   DeliveryGuarantee →  configurable per-workflow
--   QueueOp           →  operations the workflow engine can perform

module RpaElysium.Abi.ProvenQueue

import RpaElysium.Abi.Types

%default total

---------------------------------------------------------------------------
-- QueueOp — operations the workflow engine can perform on the event queue.
-- Matches proven-queueconn QueueConn.Types.QueueOp exactly.
---------------------------------------------------------------------------

||| Operations that the RPA workflow engine can perform against the
||| event queue (backed by HAR routing).
public export
data QueueOp : Type where
  ||| Publish an event (e.g., emit a derived event from a workflow step).
  Publish     : QueueOp  -- proven-queueconn tag: 0
  ||| Subscribe to receive routed events from HAR.
  Subscribe   : QueueOp  -- proven-queueconn tag: 1
  ||| Acknowledge successful processing of an automation event.
  Acknowledge : QueueOp  -- proven-queueconn tag: 2
  ||| Reject an event, optionally requesting redelivery.
  Reject      : QueueOp  -- proven-queueconn tag: 3
  ||| Inspect the next event without removing it from the queue.
  Peek        : QueueOp  -- proven-queueconn tag: 4
  ||| Remove all pending events from the queue.
  Purge       : QueueOp  -- proven-queueconn tag: 5

||| C ABI tag values — MUST match proven-queueconn encoding.
public export
queueOpTag : QueueOp -> Bits8
queueOpTag Publish     = 0
queueOpTag Subscribe   = 1
queueOpTag Acknowledge = 2
queueOpTag Reject      = 3
queueOpTag Peek        = 4
queueOpTag Purge       = 5

public export
Show QueueOp where
  show Publish     = "Publish"
  show Subscribe   = "Subscribe"
  show Acknowledge = "Acknowledge"
  show Reject      = "Reject"
  show Peek        = "Peek"
  show Purge       = "Purge"

---------------------------------------------------------------------------
-- DeliveryGuarantee — message delivery semantics, configurable per
-- workflow.
-- Matches proven-queueconn QueueConn.Types.DeliveryGuarantee exactly.
---------------------------------------------------------------------------

||| The delivery guarantee level for event processing.
||| Configurable per-workflow: idempotent workflows can use AtLeastOnce
||| while stateful workflows should use ExactlyOnce.
public export
data DeliveryGuarantee : Type where
  ||| Fire-and-forget.  Events may be lost but never duplicated.
  AtMostOnce  : DeliveryGuarantee  -- proven-queueconn tag: 0
  ||| Events are guaranteed delivered but may arrive more than once.
  AtLeastOnce : DeliveryGuarantee  -- proven-queueconn tag: 1
  ||| Events are delivered exactly once (requires idempotency or
  ||| transactional coordination).
  ExactlyOnce : DeliveryGuarantee  -- proven-queueconn tag: 2

||| C ABI tag values — MUST match proven-queueconn encoding.
public export
deliveryGuaranteeTag : DeliveryGuarantee -> Bits8
deliveryGuaranteeTag AtMostOnce  = 0
deliveryGuaranteeTag AtLeastOnce = 1
deliveryGuaranteeTag ExactlyOnce = 2

public export
Show DeliveryGuarantee where
  show AtMostOnce  = "AtMostOnce"
  show AtLeastOnce = "AtLeastOnce"
  show ExactlyOnce = "ExactlyOnce"

---------------------------------------------------------------------------
-- QueueState — the subscription lifecycle for receiving routed events
-- from HAR.
-- Matches proven-queueconn QueueConn.Types.QueueState exactly.
---------------------------------------------------------------------------

||| The lifecycle state of the event queue subscription.
public export
data QueueState : Type where
  ||| No subscription established to the event router.
  Disconnected : QueueState  -- proven-queueconn tag: 0
  ||| Subscription established and operational.
  Connected    : QueueState  -- proven-queueconn tag: 1
  ||| Actively consuming routed events from HAR.
  Consuming    : QueueState  -- proven-queueconn tag: 2
  ||| Actively producing events (e.g., derived events from workflow steps).
  Producing    : QueueState  -- proven-queueconn tag: 3
  ||| Subscription has entered a failed state.
  Failed       : QueueState  -- proven-queueconn tag: 4

||| C ABI tag values — MUST match proven-queueconn encoding.
public export
queueStateTag : QueueState -> Bits8
queueStateTag Disconnected = 0
queueStateTag Connected    = 1
queueStateTag Consuming    = 2
queueStateTag Producing    = 3
queueStateTag Failed       = 4

public export
Show QueueState where
  show Disconnected = "Disconnected"
  show Connected    = "Connected"
  show Consuming    = "Consuming"
  show Producing    = "Producing"
  show Failed       = "Failed"

---------------------------------------------------------------------------
-- MessageState — the lifecycle of an individual automation event as it
-- moves through the workflow engine's processing pipeline.
-- Matches proven-queueconn QueueConn.Types.MessageState exactly.
---------------------------------------------------------------------------

||| The lifecycle state of an individual automation event in the queue.
public export
data MessageState : Type where
  ||| The event is enqueued and awaiting delivery to a workflow.
  Pending      : MessageState  -- proven-queueconn tag: 0
  ||| The event has been delivered to a workflow but not yet acknowledged.
  Delivered    : MessageState  -- proven-queueconn tag: 1
  ||| The workflow acknowledged successful processing.
  Acknowledged : MessageState  -- proven-queueconn tag: 2
  ||| The workflow rejected the event.
  Rejected     : MessageState  -- proven-queueconn tag: 3
  ||| The event exceeded its retry limit and was moved to dead-letter.
  DeadLettered : MessageState  -- proven-queueconn tag: 4
  ||| The event's TTL has elapsed and it was discarded.
  Expired      : MessageState  -- proven-queueconn tag: 5

||| C ABI tag values — MUST match proven-queueconn encoding.
public export
messageStateTag : MessageState -> Bits8
messageStateTag Pending      = 0
messageStateTag Delivered    = 1
messageStateTag Acknowledged = 2
messageStateTag Rejected     = 3
messageStateTag DeadLettered = 4
messageStateTag Expired      = 5

public export
Show MessageState where
  show Pending      = "Pending"
  show Delivered    = "Delivered"
  show Acknowledged = "Acknowledged"
  show Rejected     = "Rejected"
  show DeadLettered = "DeadLettered"
  show Expired      = "Expired"

---------------------------------------------------------------------------
-- QueueError — error categories for event queue operations.
-- Matches proven-queueconn QueueConn.Types.QueueError exactly.
---------------------------------------------------------------------------

||| Error categories that the event queue connector can report.
public export
data QueueError : Type where
  ||| The connection to the event router was lost.
  ConnectionLost     : QueueError  -- proven-queueconn tag: 0
  ||| The specified queue does not exist.
  QueueNotFound      : QueueError  -- proven-queueconn tag: 1
  ||| The event payload exceeds the maximum allowed size.
  MessageTooLarge    : QueueError  -- proven-queueconn tag: 2
  ||| The queue or account quota has been exceeded.
  QuotaExceeded      : QueueError  -- proven-queueconn tag: 3
  ||| The acknowledgement was not received within the timeout window.
  AckTimeout         : QueueError  -- proven-queueconn tag: 4
  ||| The caller lacks permission for this queue operation.
  Unauthorized       : QueueError  -- proven-queueconn tag: 5
  ||| The event payload could not be serialised or deserialised.
  SerializationError : QueueError  -- proven-queueconn tag: 6

||| C ABI tag values — MUST match proven-queueconn encoding.
public export
queueErrorTag : QueueError -> Bits8
queueErrorTag ConnectionLost     = 0
queueErrorTag QueueNotFound      = 1
queueErrorTag MessageTooLarge    = 2
queueErrorTag QuotaExceeded      = 3
queueErrorTag AckTimeout         = 4
queueErrorTag Unauthorized       = 5
queueErrorTag SerializationError = 6

public export
Show QueueError where
  show ConnectionLost     = "ConnectionLost"
  show QueueNotFound      = "QueueNotFound"
  show MessageTooLarge    = "MessageTooLarge"
  show QuotaExceeded      = "QuotaExceeded"
  show AckTimeout         = "AckTimeout"
  show Unauthorized       = "Unauthorized"
  show SerializationError = "SerializationError"

---------------------------------------------------------------------------
-- Valid queue state transitions — proof-carrying types that ensure
-- only legal subscription lifecycle transitions can be constructed.
---------------------------------------------------------------------------

||| Proof that a queue state transition from `from` to `to` is valid.
public export
data ValidQueueTransition : (from : QueueState) -> (to : QueueState) -> Type where
  ||| Disconnected → Connected: establish subscription to HAR.
  Connect         : ValidQueueTransition Disconnected Connected
  ||| Connected → Consuming: begin consuming routed events.
  StartConsuming  : ValidQueueTransition Connected Consuming
  ||| Connected → Producing: begin producing derived events.
  StartProducing  : ValidQueueTransition Connected Producing
  ||| Consuming → Connected: stop consuming but keep connection.
  StopConsuming   : ValidQueueTransition Consuming Connected
  ||| Producing → Connected: stop producing but keep connection.
  StopProducing   : ValidQueueTransition Producing Connected
  ||| Connected → Disconnected: cleanly disconnect.
  Disconnect      : ValidQueueTransition Connected Disconnected
  ||| Any → Failed: connection failure (from Connected).
  FailConnected   : ValidQueueTransition Connected Failed
  ||| Any → Failed: connection failure (from Consuming).
  FailConsuming   : ValidQueueTransition Consuming Failed
  ||| Any → Failed: connection failure (from Producing).
  FailProducing   : ValidQueueTransition Producing Failed
  ||| Failed → Disconnected: reset after failure.
  ResetFailed     : ValidQueueTransition Failed Disconnected

public export
Show (ValidQueueTransition from to) where
  show Connect        = "Connect (Disconnected → Connected)"
  show StartConsuming = "StartConsuming (Connected → Consuming)"
  show StartProducing = "StartProducing (Connected → Producing)"
  show StopConsuming  = "StopConsuming (Consuming → Connected)"
  show StopProducing  = "StopProducing (Producing → Connected)"
  show Disconnect     = "Disconnect (Connected → Disconnected)"
  show FailConnected  = "FailConnected (Connected → Failed)"
  show FailConsuming  = "FailConsuming (Consuming → Failed)"
  show FailProducing  = "FailProducing (Producing → Failed)"
  show ResetFailed    = "ResetFailed (Failed → Disconnected)"
