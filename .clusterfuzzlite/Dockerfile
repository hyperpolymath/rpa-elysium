# SPDX-License-Identifier: MPL-2.0
# ClusterFuzzLite build image for rpa-elysium (language: rust).
#
# Per ClusterFuzzLite's build-integration docs, this file MUST be named
# `Dockerfile` inside `.clusterfuzzlite/` — the build action looks for
# exactly that path. The repo previously shipped the same content as
# `.clusterfuzzlite/Containerfile`, which the action never reads; every
# fuzz run failed with "failed to read dockerfile: open Dockerfile: no
# such file or directory".
FROM gcr.io/oss-fuzz-base/base-builder-rust

RUN apt-get update && apt-get install -y make autoconf automake libtool

# base-builder-rust ships the pinned nightly toolchain and cargo-fuzz
# pre-installed (see CFL docs, "Integrating a Rust project"); nothing
# else to install for this project.

COPY . $SRC/project
WORKDIR $SRC/project

COPY .clusterfuzzlite/build.sh $SRC/
