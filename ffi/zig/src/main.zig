// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// main.zig — C-compatible FFI implementation for RPA Elysium
//
// Implements the ABI declared in src/abi/Foreign.idr.
// All exported functions use C calling convention for maximum compatibility.

const std = @import("std");

/// Library version — must match Cargo.toml
const VERSION = "0.1.0";

// ============================================================
// Type definitions (mirror src/abi/Types.idr)
// ============================================================

/// Event kind tags (matches Idris2 eventKindTag)
pub const EventKind = enum(u8) {
    file_created = 0,
    file_modified = 1,
    file_deleted = 2,
    file_renamed = 3,
    manual = 4,
    scheduled = 5,
};

/// Timestamp (matches Idris2 Timestamp record)
pub const Timestamp = extern struct {
    seconds: i64,
    nanos: u32,
    _padding: u32 = 0,
};

/// Workflow status (matches Idris2 WorkflowStatus)
pub const WorkflowStatus = enum(u8) {
    idle = 0,
    running = 1,
    paused = 2,
    stopped = 3,
    err = 4,
};

/// Action result (matches Idris2 ActionResult)
pub const ActionResult = extern struct {
    success: bool,
    _padding: [7]u8 = .{0} ** 7,
    message: ?[*:0]const u8,
};

/// Error codes (matches Idris2 errorCode)
pub const ErrorCode = enum(u32) {
    ok = 0,
    io = 1,
    config = 2,
    workflow = 3,
    action_failed = 4,
    invalid_pattern = 5,
    watch = 6,
};

/// Opaque workflow handle
pub const WorkflowHandle = opaque {};

// ============================================================
// Thread-local error state
// ============================================================

threadlocal var last_error: ?[]const u8 = null;

fn setError(msg: []const u8) void {
    last_error = msg;
}

// ============================================================
// Exported FFI functions (C ABI)
// ============================================================

/// Create a new workflow instance.
/// Returns an opaque handle, or null on error.
export fn rpa_workflow_create(name: ?[*:0]const u8) callconv(.C) ?*WorkflowHandle {
    _ = name;
    // TODO: Implement — allocate and initialise a WorkflowRunner
    setError("rpa_workflow_create not yet implemented");
    return null;
}

/// Destroy a workflow instance and free all associated resources.
export fn rpa_workflow_destroy(handle: ?*WorkflowHandle) callconv(.C) void {
    _ = handle;
    // TODO: Implement — tear down and deallocate
}

/// Query the current status of a workflow.
/// Returns a WorkflowStatus tag byte.
export fn rpa_workflow_status(handle: ?*WorkflowHandle) callconv(.C) u8 {
    _ = handle;
    // TODO: Implement — query actual state
    return @intFromEnum(WorkflowStatus.idle);
}

/// Process an event through the workflow engine.
/// Returns 0 on success, or an ErrorCode on failure.
export fn rpa_event_process(handle: ?*WorkflowHandle, event: ?*anyopaque) callconv(.C) i32 {
    _ = handle;
    _ = event;
    // TODO: Implement — dispatch event to matching rules
    setError("rpa_event_process not yet implemented");
    return @intCast(@intFromEnum(ErrorCode.workflow));
}

/// Copy the last error message into the provided buffer.
/// Returns the number of bytes written, or -1 if no error is pending.
export fn rpa_last_error(buf: ?[*]u8, buf_len: usize) callconv(.C) i32 {
    const err = last_error orelse return -1;
    if (buf) |b| {
        const copy_len = @min(err.len, buf_len -| 1);
        @memcpy(b[0..copy_len], err[0..copy_len]);
        b[copy_len] = 0;
        return @intCast(copy_len);
    }
    return -1;
}

/// Return the library version string.
export fn rpa_version() callconv(.C) [*:0]const u8 {
    return VERSION;
}

// ============================================================
// Tests
// ============================================================

test "version returns correct string" {
    const v = rpa_version();
    const slice = std.mem.span(v);
    try std.testing.expectEqualStrings("0.1.0", slice);
}

test "workflow_create returns null (stub)" {
    const handle = rpa_workflow_create("test");
    try std.testing.expect(handle == null);
}

test "last_error returns message after failed create" {
    _ = rpa_workflow_create("test");
    var buf: [256]u8 = undefined;
    const len = rpa_last_error(&buf, buf.len);
    try std.testing.expect(len > 0);
}

test "workflow_status returns idle for null handle" {
    const status = rpa_workflow_status(null);
    try std.testing.expectEqual(@as(u8, 0), status);
}
