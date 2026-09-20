# SPDX-License-Identifier: MPL-2.0
# ClusterFuzzLite build image for rpa-elysium (language: rust).
#
# ClusterFuzzLite's build_fuzzers action requires this file at the repository
# ROOT, named exactly `Dockerfile`. A previous copy lived at
# .clusterfuzzlite/Containerfile, which the action never reads — every fuzz
# run failed with "failed to read dockerfile: open Dockerfile: no such file
# or directory". This file supersedes it.
FROM gcr.io/oss-fuzz-base/base-builder-rust

RUN apt-get update && apt-get install -y make autoconf automake libtool

# base-builder-rust ships the pinned nightly toolchain (RUSTUP_TOOLCHAIN) but
# not cargo-fuzz itself; install it so .clusterfuzzlite/build.sh can run
# `cargo fuzz build`.
RUN cargo install cargo-fuzz --locked

COPY . $SRC/project
WORKDIR $SRC/project

COPY .clusterfuzzlite/build.sh $SRC/
