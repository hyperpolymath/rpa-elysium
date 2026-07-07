// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
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

    // The root module shared by every artifact (Zig 0.15 build API).
    const mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
        // main.zig uses std.heap.c_allocator for C-ABI-owned allocations.
        .link_libc = true,
    });

    // Shared library for FFI consumers.
    const lib = b.addLibrary(.{
        .name = "rpa_ffi",
        .root_module = mod,
        .linkage = .dynamic,
    });

    // Also produce a static library for embedding.
    const static_lib = b.addLibrary(.{
        .name = "rpa_ffi",
        .root_module = mod,
        .linkage = .static,
    });

    // Install both artifacts.
    b.installArtifact(lib);
    b.installArtifact(static_lib);

    // Unit tests.
    const main_tests = b.addTest(.{ .root_module = mod });

    const run_main_tests = b.addRunArtifact(main_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_main_tests.step);
}
