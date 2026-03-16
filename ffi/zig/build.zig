// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// build.zig — Zig build configuration for the RPA Elysium FFI layer
//
// Produces a C-compatible shared library (librpa_ffi) that implements
// the ABI defined in src/abi/*.idr.

const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Shared library for FFI consumers
    const lib = b.addSharedLibrary(.{
        .name = "rpa_ffi",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    // Also produce a static library for embedding
    const static_lib = b.addStaticLibrary(.{
        .name = "rpa_ffi",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    // Install both artifacts
    b.installArtifact(lib);
    b.installArtifact(static_lib);

    // Unit tests
    const main_tests = b.addTest(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    const run_main_tests = b.addRunArtifact(main_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_main_tests.step);
}
