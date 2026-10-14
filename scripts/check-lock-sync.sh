#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Offline, observational validation of direct refs AND transitive lock closure.
# Parse YAML, not lines: quoted/flow YAML is legal; run bodies are not uses.
# Requires mikefarah/yq v4 and jq (available on GitHub Ubuntu runners).
set -euo pipefail
WF_DIR="${1:-.github/workflows}"
YQ_BIN="${YQ_BIN:-yq}"
command -v "$YQ_BIN" >/dev/null || { echo '::error::mikefarah/yq v4 is required'; exit 2; }
command -v jq >/dev/null || { echo '::error::jq is required'; exit 2; }
[ -f "$WF_DIR/actions.lock" ] || { echo '::error::actions.lock missing'; exit 1; }
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
# Fail closed on tool failure, malformed YAML or unsupported schema.
"$YQ_BIN" -o=json '.' "$WF_DIR/actions.lock" > "$scratch/lock.json"
jq -e '(.version == "v0.0.2" or .version == "v0.0.3") and (.workflows | type == "object") and (.dependencies | type == "object") and
  all(.dependencies | to_entries[];
    (.key | test("^[^/@]+/[^/@]+@[^@[:space:]]+$")) and
    (.value | type == "object") and
    (.value.owner_id | type == "number") and (.value.owner_id > 0) and
    (.value.repo_id | type == "number") and (.value.repo_id > 0))' "$scratch/lock.json" >/dev/null
shopt -s nullglob
files=("$WF_DIR"/*.yml "$WF_DIR"/*.yaml)
[ ${#files[@]} -gt 0 ] || { echo '::error::No workflows found'; exit 1; }
for file in "${files[@]}"; do
  "$YQ_BIN" -o=json '.' "$file" > "$scratch/workflow.json"
  jq -e 'type == "object" and (.jobs | type == "object" and length > 0)' "$scratch/workflow.json" >/dev/null
  jq --arg path ".github/workflows/$(basename "$file")" '
    {path: $path, refs: [.jobs[] | .uses?, (.steps[]? | .uses?) | select(. != null)]}
  ' "$scratch/workflow.json" >> "$scratch/refs.jsonl"
done
jq -s '.' "$scratch/refs.jsonl" > "$scratch/refs.json"
jq -L "$(dirname "${BASH_SOURCE[0]}")" -n --slurpfile lock "$scratch/lock.json" --slurpfile workflows "$scratch/refs.json" '
  include "lock-graph";
  $lock[0] as $l | $workflows[0] as $ws |
  ($l.dependencies | normal_dependencies) as $deps |
  reachable([$l.workflows[] | .[]]; $deps) as $live |
  [
    ($ws[] | . as $w | ($w.refs | refs) as $want |
      ($l.workflows[$w.path] // [] | refs) as $got |
      if $want != $got then {type: "lock-drift", workflow: $w.path, missing: ($want - $got), stale: ($got - $want)} else empty end),
    ($l.workflows | keys[] | . as $p | select(all($ws[]; .path != $p)) | {type: "orphan-workflow", workflow: $p}),
    (((($l.workflows[] | refs[]), ($l.dependencies[] | (.uses // []) | refs[])) as $ref |
      select($deps[$ref] == null) | {type: "dangling-edge", ref: $ref})),
    ($deps | to_entries[] | select((.value.commit | type != "string") or ((.value.commit // "") | test("^sha1-[0-9a-f]{40}$") | not)) |
      {type: "invalid-commit", ref: .key})
    , (($deps | keys) - $live | .[] | {type: "unreferenced-dependency", ref: .})
  ] | unique
' > "$scratch/findings.json"
if ! jq -e 'length == 0' "$scratch/findings.json" >/dev/null; then
  jq '.' "$scratch/findings.json"
  echo '::error::actions.lock is out of sync or not transitively closed. Update and verify it in the same PR as uses changes.'
  exit 1
fi
echo 'actions.lock: direct refs match, no orphan workflows, immutable commits, transitive closure verified.'
