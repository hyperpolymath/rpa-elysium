-- SPDX-License-Identifier: MPL-2.0
-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
||| SPDX-License-Identifier: MPL-2.0
||| Foreign Function Interface Declarations for RPA_ELYSIUM
|||
||| This module declares all C-compatible functions that will be
||| implemented in the Zig FFI layer.
|||
||| All functions are declared here with type signatures and safety proofs.
||| Implementations live in ffi/zig/

module Foreign

import Types
import Layout

%default total

--------------------------------------------------------------------------------
-- FFI glue types (must match ffi/zig and the generated C header)
--------------------------------------------------------------------------------

||| FFI result codes returned across the C ABI. Tag values MUST match the Zig
||| implementation (ffi/zig): Ok=0, Error=1, InvalidParam=2, OutOfMemory=3,
||| NullPointer=4.
public export
data Result : Type where
  Ok           : Result
  Error        : Result
  InvalidParam : Result
  OutOfMemory  : Result
  NullPointer  : Result

public export
Show Result where
  show Ok           = "Ok"
  show Error        = "Error"
  show InvalidParam = "InvalidParam"
  show OutOfMemory  = "OutOfMemory"
  show NullPointer  = "NullPointer"

||| The C-ABI tag byte for a result code.
public export
resultTag : Result -> Bits32
resultTag Ok           = 0
resultTag Error        = 1
resultTag InvalidParam = 2
resultTag OutOfMemory  = 3
resultTag NullPointer  = 4

||| An opaque handle to a library instance, wrapping the raw C pointer as a
||| Bits64. Constructed only via `createHandle`, which rejects the null pointer.
public export
data Handle : Type where
  MkHandle : Bits64 -> Handle

||| Construct a handle from a raw pointer; `Nothing` for the null pointer.
public export
createHandle : Bits64 -> Maybe Handle
createHandle 0 = Nothing
createHandle p = Just (MkHandle p)

||| The raw C pointer backing a handle.
public export
handlePtr : Handle -> Bits64
handlePtr (MkHandle p) = p

--------------------------------------------------------------------------------
-- Library Lifecycle
--------------------------------------------------------------------------------

||| Initialize the library
||| Returns a handle to the library instance, or Nothing on failure
export
%foreign "C:rpa_elysium_init, librpa_elysium"
prim__init : PrimIO Bits64

||| Safe wrapper for library initialization
export
init : IO (Maybe Handle)
init = do
  ptr <- primIO prim__init
  pure (createHandle ptr)

||| Clean up library resources
export
%foreign "C:rpa_elysium_free, librpa_elysium"
prim__free : Bits64 -> PrimIO ()

||| Safe wrapper for cleanup
export
free : Handle -> IO ()
free h = primIO (prim__free (handlePtr h))

--------------------------------------------------------------------------------
-- Core Operations
--------------------------------------------------------------------------------

||| Example operation: process data
export
%foreign "C:rpa_elysium_process, librpa_elysium"
prim__process : Bits64 -> Bits32 -> PrimIO Bits32

||| Safe wrapper with error handling
export
process : Handle -> Bits32 -> IO (Either Result Bits32)
process h input = do
  result <- primIO (prim__process (handlePtr h) input)
  pure $ case result of
    0 => Left Error
    n => Right n

--------------------------------------------------------------------------------
-- String Operations
--------------------------------------------------------------------------------

||| Convert C string to Idris String
export
%foreign "support:idris2_getString, libidris2_support"
prim__getString : Bits64 -> String

||| Free C string
export
%foreign "C:rpa_elysium_free_string, librpa_elysium"
prim__freeString : Bits64 -> PrimIO ()

||| Get string result from library
export
%foreign "C:rpa_elysium_get_string, librpa_elysium"
prim__getResult : Bits64 -> PrimIO Bits64

||| Safe string getter
export
getString : Handle -> IO (Maybe String)
getString h = do
  ptr <- primIO (prim__getResult (handlePtr h))
  if ptr == 0
    then pure Nothing
    else do
      let str = prim__getString ptr
      primIO (prim__freeString ptr)
      pure (Just str)

--------------------------------------------------------------------------------
-- Array/Buffer Operations
--------------------------------------------------------------------------------

||| Process array data
export
%foreign "C:rpa_elysium_process_array, librpa_elysium"
prim__processArray : Bits64 -> Bits64 -> Bits32 -> PrimIO Bits32

||| Safe array processor
export
processArray : Handle -> (buffer : Bits64) -> (len : Bits32) -> IO (Either Result ())
processArray h buf len = do
  result <- primIO (prim__processArray (handlePtr h) buf len)
  pure $ case resultFromInt result of
    Just Ok => Right ()
    Just err => Left err
    Nothing => Left Error
  where
    resultFromInt : Bits32 -> Maybe Result
    resultFromInt 0 = Just Ok
    resultFromInt 1 = Just Error
    resultFromInt 2 = Just InvalidParam
    resultFromInt 3 = Just OutOfMemory
    resultFromInt 4 = Just NullPointer
    resultFromInt _ = Nothing

--------------------------------------------------------------------------------
-- Error Handling
--------------------------------------------------------------------------------

||| Get last error message
export
%foreign "C:rpa_elysium_last_error, librpa_elysium"
prim__lastError : PrimIO Bits64

||| Retrieve last error as string
export
lastError : IO (Maybe String)
lastError = do
  ptr <- primIO prim__lastError
  if ptr == 0
    then pure Nothing
    else pure (Just (prim__getString ptr))

||| Get error description for result code
export
errorDescription : Result -> String
errorDescription Ok = "Success"
errorDescription Error = "Generic error"
errorDescription InvalidParam = "Invalid parameter"
errorDescription OutOfMemory = "Out of memory"
errorDescription NullPointer = "Null pointer"

--------------------------------------------------------------------------------
-- Version Information
--------------------------------------------------------------------------------

||| Get library version
export
%foreign "C:rpa_elysium_version, librpa_elysium"
prim__version : PrimIO Bits64

||| Get version as string
export
version : IO String
version = do
  ptr <- primIO prim__version
  pure (prim__getString ptr)

||| Get library build info
export
%foreign "C:rpa_elysium_build_info, librpa_elysium"
prim__buildInfo : PrimIO Bits64

||| Get build information
export
buildInfo : IO String
buildInfo = do
  ptr <- primIO prim__buildInfo
  pure (prim__getString ptr)

--------------------------------------------------------------------------------
-- Callback Support
--------------------------------------------------------------------------------

||| Callback function type (C ABI) — the shape a registered callback must
||| have: `(context : Bits64) -> (event : Bits32) -> (result : Bits32)`.
public export
Callback : Type
Callback = Bits64 -> Bits32 -> Bits32

||| Register a callback
export
%foreign "C:rpa_elysium_register_callback, librpa_elysium"
prim__registerCallback : Bits64 -> Bits64 -> PrimIO Bits32

||| Safe callback registration. The callback is passed as a raw pointer to a
||| C-callable function of type `Callback`: an Idris closure cannot be handed to
||| C directly (that needs a foreign export), so the caller supplies the
||| already-C-callable function pointer (`cbPtr`).
export
registerCallback : Handle -> (cbPtr : Bits64) -> IO (Either Result ())
registerCallback h cbPtr = do
  result <- primIO (prim__registerCallback (handlePtr h) cbPtr)
  pure $ case resultFromInt result of
    Just Ok => Right ()
    Just err => Left err
    Nothing => Left Error
  where
    resultFromInt : Bits32 -> Maybe Result
    resultFromInt 0 = Just Ok
    resultFromInt _ = Just Error

--------------------------------------------------------------------------------
-- Utility Functions
--------------------------------------------------------------------------------

||| Check if library is initialized
export
%foreign "C:rpa_elysium_is_initialized, librpa_elysium"
prim__isInitialized : Bits64 -> PrimIO Bits32

||| Check initialization status
export
isInitialized : Handle -> IO Bool
isInitialized h = do
  result <- primIO (prim__isInitialized (handlePtr h))
  pure (result /= 0)
