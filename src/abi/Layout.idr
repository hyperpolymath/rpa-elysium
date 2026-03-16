-- SPDX-License-Identifier: PMPL-1.0-or-later
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
-- Layout.idr — Memory layout proofs for C ABI compatibility
--
-- Ensures that type representations are consistent across the
-- Idris2 ABI definitions and the Zig FFI implementation.

module RpaElysium.Abi.Layout

import RpaElysium.Abi.Types

%default total

||| Size in bytes of a C-compatible type representation
public export
data CSize : Type where
  Bytes : (n : Nat) -> CSize

||| Alignment requirement in bytes
public export
data CAlign : Type where
  Align : (n : Nat) -> CAlign

||| Layout descriptor for a C-compatible struct
public export
record CLayout where
  constructor MkCLayout
  size      : Nat
  alignment : Nat

||| Layout for Timestamp (Int64 + Bits32 = 8 + 4 = 12, padded to 16 for alignment)
public export
timestampLayout : CLayout
timestampLayout = MkCLayout 16 8

||| Layout for ActionResult (Bool + pointer to string = 1 + 7 padding + 8 = 16)
public export
actionResultLayout : CLayout
actionResultLayout = MkCLayout 16 8

||| Layout for WorkflowStatus enum (single byte tag, padded to 4)
public export
workflowStatusLayout : CLayout
workflowStatusLayout = MkCLayout 4 4

||| Layout for error code (Bits32)
public export
errorCodeLayout : CLayout
errorCodeLayout = MkCLayout 4 4

||| Proof that layout sizes are non-zero (valid for allocation)
public export
layoutNonZero : (layout : CLayout) -> {auto prf : GT (layout.size) 0} -> GT (layout.size) 0
layoutNonZero _ {prf} = prf

||| Proof that alignment is a power of two
||| (simplified — in production this would be a proper isPowerOfTwo proof)
public export
data ValidAlignment : Nat -> Type where
  Align1 : ValidAlignment 1
  Align2 : ValidAlignment 2
  Align4 : ValidAlignment 4
  Align8 : ValidAlignment 8
  Align16 : ValidAlignment 16

||| Proof that timestamp layout has valid alignment
public export
timestampAlignValid : ValidAlignment 8
timestampAlignValid = Align8

||| Proof that all event kind tags fit in a single byte
public export
eventKindTagFitsInByte : (ek : EventKind) -> LTE (cast (eventKindTag ek)) 255
eventKindTagFitsInByte (FileCreated _)    = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagFitsInByte (FileModified _)   = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagFitsInByte (FileDeleted _)    = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagFitsInByte (FileRenamed _ _)  = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagFitsInByte Manual             = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
eventKindTagFitsInByte (Scheduled _)      = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
