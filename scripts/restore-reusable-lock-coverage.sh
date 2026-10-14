#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# gh-actions-lock does not discover job-level reusable refs. Preserve the
# already-resolved graph and restore exact caller refs after regeneration.
# New refs still need resolved records; check-lock-sync fails closed on those.
set -euo pipefail
old=$1
wf_dir=${2:-.github/workflows}
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
yq -o=json '.' "$wf_dir/actions.lock" > "$scratch/current.json"
if [ -f "$old" ]; then
  yq -o=json '.' "$old" > "$scratch/old.json"
else
  echo '{"dependencies":{}}' > "$scratch/old.json"
fi
shopt -s nullglob
for file in "$wf_dir"/*.yml "$wf_dir"/*.yaml; do
  yq -o=json '.' "$file" | jq --arg path ".github/workflows/$(basename "$file")" '
    .jobs[] | .uses? | select(type == "string") |
    select(test("^[^/@]+/[^/@]+/\\.github/workflows/[^@]+@[^@[:space:]]+$")) |
    capture("^(?<owner>[^/]+)/(?<repo>[^/]+)/[^@]+@(?<ref>.+)$") |
    {workflow: $path, dependency: (.owner + "/" + .repo + "@" + .ref)}
  ' >> "$scratch/reusables.jsonl"
done
jq -s '.' "$scratch/reusables.jsonl" > "$scratch/reusables.json"
jq -L "$(dirname "${BASH_SOURCE[0]}")" --slurpfile old "$scratch/old.json" --slurpfile refs "$scratch/reusables.json" '
  include "lock-graph";
  ($old[0].dependencies | normal_dependencies) as $olddeps |
  reachable([$refs[0][].dependency]; $olddeps) as $keep |
  .dependencies = (($olddeps | with_entries(select(.key as $k | $keep | index($k)))) + (.dependencies | normal_dependencies)) |
  reduce $refs[0][] as $r (.; .workflows[$r.workflow] = ((.workflows[$r.workflow] // []) + [$r.dependency] | unique)) |
  # A regenerated graph may contain obsolete direct records too. Prune only
  # after roots are restored; never drop a reachable transitive record.
  reachable([.workflows[] | .[]]; .dependencies) as $live |
  .dependencies |= with_entries(select(.key as $k | $live | index($k)))
' "$scratch/current.json" > "$scratch/merged.json"
# Write only when parsing and merging both succeeded. The parent updater owns
# rollback if its subsequent authoritative or structural verification fails.
yq -P -o=yaml '.' "$scratch/merged.json" > "$scratch/actions.lock"
cp "$scratch/actions.lock" "$wf_dir/actions.lock"
