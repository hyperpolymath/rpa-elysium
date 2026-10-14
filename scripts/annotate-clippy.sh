#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Preserve clippy's exit status and expose diagnostics through the Checks API.
# This is an observability aid, never a waiver for -D warnings.
set -euo pipefail
messages=$(mktemp)
trap 'rm -f "$messages"' EXIT
status=0
cargo clippy --locked --all-targets --message-format=json -- -D warnings > "$messages" || status=$?
cat "$messages"
jq -r '
  def escape: tostring | gsub("%"; "%25") | gsub("\r"; "%0D") | gsub("\n"; "%0A");
  def property: escape | gsub(","; "%2C") | gsub(":"; "%3A");
  select(.reason == "compiler-message") | .message as $m |
  select($m.level == "error" or $m.level == "warning") |
  ($m.spans | map(select(.is_primary)) | first // {}) as $s |
  "::error file=\(($s.file_name // "Cargo.toml") | property),line=\($s.line_start // 1)::\(($m.rendered // $m.message) | escape)"
' "$messages"
exit "$status"
