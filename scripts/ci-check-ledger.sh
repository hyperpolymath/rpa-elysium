#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# ci-check-ledger.sh — every workflow in this repository has a recorded state,
# and every workflow that is not green has a written determination.
#
# This is the mechanised form of the estate's standing stopping rule: a red
# check is an issue with acceptance criteria, never something that stays
# undetermined (hyperpolymath/standards#138, #1070). Reds used to accumulate
# invisibly here — five workflows sat red or dormant on main while the only
# tracked triage issue named one check.
#
# The ledger (docs/audits/ci-check-ledger.json) is the single place where a
# determination is written. --check requires it to cover exactly the workflow
# files on disk, so neither a new workflow nor a deleted one can be missed.
#
# States, derived in this order:
#   dormant    the workflow is disabled in GitHub (a deliberate owner act)
#   green      the latest completed run on the default branch succeeded
#   red        the latest completed run failed
#   stale      the latest completed run failed/cancelled but predates the last
#              commit that touched the workflow file, so it measures an older
#              revision (repair is in the tree, awaiting a run)
#   superseded the latest completed run was cancelled or skipped
#   never-run  no completed run on the default branch
#   unknown    the API reported something this script does not classify
#
# Tiering: green needs evidence and a green|fixed determination. red is the
# strict tier and needs reason, owner_action, an issue URL and review_by.
# Every other non-green state is the light tier and needs reason and review_by
# plus at least one of owner_action / issue.
#
# Modes:
#   --check   (default) offline, fail-closed structural validation.
#   --verify  compare recorded states with the latest runs on the default
#             branch. Fails when a workflow recorded as green is not: an
#             unrecorded red is exactly what this mechanism exists to prevent.
#             Benign transitions warn; recording them is bookkeeping.
#   --write   update states, conclusions, evidence and staleness from the API.
#             Human judgement (determination, reason, owner_action, issue,
#             review_by) is never touched.
#
# Usage:
#   scripts/ci-check-ledger.sh [--check|--verify|--write] [REPO_ROOT]
#                              [--ledger FILE] [--observations FILE]
#                              [--repo OWNER/NAME] [--branch NAME] [--quiet]
# Exit: 0 pass | 1 violation | 2 setup failure

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/.."
# An optional positional argument (never a flag) names the repository root.
if [ "$#" -gt 0 ] && [ -d "${1:-}" ] && [ "${1#-}" = "$1" ]; then
  REPO_ROOT="$1"
  shift
fi
REPO_ROOT="$(cd "$REPO_ROOT" && pwd)"

LEDGER="docs/audits/ci-check-ledger.json"
MODE="check"
OBSERVATIONS=""
REPO="hyperpolymath/rpa-elysium"
BRANCH="main"
WORKFLOW_REL=".github/workflows"
QUIET=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --check)        MODE="check" ;;
    --verify)       MODE="verify" ;;
    --write)        MODE="write" ;;
    --ledger)       LEDGER="$2"; shift ;;
    --observations) OBSERVATIONS="$2"; shift ;;
    --repo)         REPO="$2"; shift ;;
    --branch)       BRANCH="$2"; shift ;;
    --quiet)        QUIET=1 ;;
    -h|--help)      sed -n '2,45p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) printf 'ci-check-ledger: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

case "$LEDGER" in /*) ;; *) LEDGER="$REPO_ROOT/$LEDGER" ;; esac
case "$OBSERVATIONS" in ""|/*) ;; *) OBSERVATIONS="$REPO_ROOT/$OBSERVATIONS" ;; esac

say()  { [ "$QUIET" -eq 1 ] || printf 'ci-check-ledger: %s\n' "$*"; }
warn() { printf '::warning::ci-check-ledger: %s\n' "$*" >&2; }
bad()  { printf 'ci-check-ledger: FAIL: %s\n' "$*" >&2; FAILED=1; }
note() { printf 'ci-check-ledger: %s\n' "$*"; }

FAILED=0
STATES='["green","red","stale","superseded","dormant","never-run","unknown"]'
DETERMINATIONS='["green","fixed","exempt","in-triage"]'
STRICT_STATES='["red"]'
LIGHT_STATES='["stale","superseded","dormant","never-run","unknown"]'

# ------------------------------------------------------------------- setup
command -v jq >/dev/null 2>&1 || { printf 'ci-check-ledger: jq is required\n' >&2; exit 2; }
[ -f "$LEDGER" ] || { printf 'ci-check-ledger: ledger not found: %s\n' "$LEDGER" >&2; exit 2; }
jq -e . "$LEDGER" >/dev/null 2>&1 || { printf 'ci-check-ledger: ledger is not valid JSON: %s\n' "$LEDGER" >&2; exit 2; }

mapfile -t FILES < <(cd "$REPO_ROOT" && ls "$WORKFLOW_REL"/*.yml "$WORKFLOW_REL"/*.yaml 2>/dev/null | sort)
[ "${#FILES[@]}" -gt 0 ] || { printf 'ci-check-ledger: no workflows found under %s\n' "$WORKFLOW_REL" >&2; exit 2; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
printf '%s\n' "${FILES[@]}" > "$TMP/disk-paths"
jq -r '.workflows[].path' "$LEDGER" | sort > "$TMP/ledger-paths"

TODAY="$(date -u +%F)"
HORIZON="$(date -u -d '+90 days' +%F 2>/dev/null || date -u -v+90d +%F)"

# ------------------------------------------------------- observations source
observed_states() {
  # Emits {"<path>": {"workflow_state", "conclusion", "run", "observed",
  # "file_updated"}} for every workflow, or writes a reason to stdout and
  # returns non-zero when observation is impossible.
  if [ -n "$OBSERVATIONS" ]; then
    jq -e . "$OBSERVATIONS" >/dev/null 2>&1 || { echo "observations file is not valid JSON: $OBSERVATIONS"; return 2; }
    jq -c . "$OBSERVATIONS" > "$TMP/obs.final.json"
    emit_observations
    return $?
  fi
  command -v gh >/dev/null 2>&1 || { echo "gh is not installed"; return 2; }

  if ! gh api "repos/$REPO/actions/workflows?per_page=100" \
       > "$TMP/workflows.json" 2> "$TMP/err"; then
    echo "GitHub API unavailable for the workflow list: $(head -1 "$TMP/err" | cut -c1-140)"
    return 2
  fi
  if ! gh api "repos/$REPO/actions/runs?branch=$BRANCH&per_page=100" --paginate \
       > "$TMP/runs.jsonl" 2> "$TMP/err"; then
    echo "GitHub API unavailable for the run list: $(head -1 "$TMP/err" | cut -c1-140)"
    return 2
  fi
  [ -s "$TMP/runs.jsonl" ] || { echo "GitHub API returned no run pages for $REPO ($BRANCH)"; return 2; }

  # --paginate emits one JSON document per page; -s slurps them into an array.
  # The newest COMPLETED run is used, so an in-flight run is never mistaken for
  # a state change; a workflow disabled in GitHub is recorded as dormant.
  jq -c -s --slurpfile wf "$TMP/workflows.json" '
    ($wf[0].workflows | map({key: (.path | sub("^/";"")), value: .state}) | from_entries) as $states
    | [.[] | .workflow_runs[]?]
    | map(select(.path != null))
    | group_by(.path)
    | map(
        (map(select(.conclusion != null)) | max_by(.created_at)) as $done
        | {key: (.[0].path | sub("^/";"")),
           value: (if $done == null
                   then {workflow_state: ($states[.[0].path] // "active"), conclusion: null,
                         run: "", observed: null, file_updated: null}
                   else {workflow_state: ($states[$done.path] // "active"), conclusion: $done.conclusion,
                         run: ($done.html_url // ""), observed: ($done.created_at | .[0:10]),
                         file_updated: null}
                   end)}
      )
    | from_entries
  ' "$TMP/runs.jsonl" > "$TMP/obs.json"

  # Staleness only matters for observations that are not green and not dormant:
  # compare the run date with the last commit that touched the workflow file.
  jq -r 'to_entries[]
         | select(.value.workflow_state == "active")
         | select(.value.conclusion != null and .value.conclusion != "success")
         | select(.value.conclusion != "skipped")
         | .key' "$TMP/obs.json" > "$TMP/needs-staleness"
  : > "$TMP/staleness.jsonl"
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    when="$(gh api "repos/$REPO/commits?sha=$BRANCH&path=$path&per_page=1" \
              --jq 'if length > 0 then .[0].commit.committer.date else null end' 2>/dev/null || echo null)"
    when="${when%\"}"; when="${when#\"}"
    [ -n "$when" ] && [ "$when" != "null" ] || continue
    jq -cn --arg p "$path" --arg d "${when:0:10}" '{key: $p, value: $d}' >> "$TMP/staleness.jsonl"
  done < "$TMP/needs-staleness"

  if [ -s "$TMP/staleness.jsonl" ]; then
    # No -s here: slurping the observations would turn the object into an array
    # and with_entries would index it numerically, silently emptying every
    # lookup. --slurpfile already handles the dates.
    jq -c --slurpfile dates "$TMP/staleness.jsonl" '
      # --slurpfile already yields an array of the JSON values in the file.
      ($dates | from_entries) as $d
      | with_entries(.value.file_updated = ($d[.key] // null))
    ' "$TMP/obs.json" > "$TMP/obs.final.json" || { echo "could not merge staleness into the observations"; return 2; }
  else
    cp "$TMP/obs.json" "$TMP/obs.final.json"
  fi
  emit_observations
}

# emit_observations is the single exit path: fail closed on an empty or
# malformed set, because every lookup would then read null and every workflow
# would be recorded as never-run — worse than a hard error.
emit_observations() {
  if ! jq -e 'type == "object" and (length > 0)
              and all(.[]; type == "object" and (.conclusion == null or (.conclusion | type == "string")))' \
       "$TMP/obs.final.json" >/dev/null 2>&1; then
    echo "the observation set is empty or malformed"
    return 2
  fi
  cat "$TMP/obs.final.json"
  return 0
}

# classify lives in one place so --check, --verify and --write cannot disagree.
read -r -d '' CLASSIFY <<'JQ' || true
def classify($o):
  if $o == null then "never-run"
  elif (($o.workflow_state // "active") | startswith("disabled")) then "dormant"
  elif $o.conclusion == "success" then "green"
  elif ($o.conclusion == "failure" or $o.conclusion == "startup_failure"
        or $o.conclusion == "timed_out" or $o.conclusion == "action_required") then
    (if ($o.file_updated != null and $o.observed != null and $o.file_updated > $o.observed)
     then "stale" else "red" end)
  elif ($o.conclusion == "cancelled" or $o.conclusion == "skipped") then
    (if ($o.file_updated != null and $o.observed != null and $o.file_updated > $o.observed)
     then "stale" else "superseded" end)
  elif ($o.conclusion == null) then "never-run"
  else "unknown" end;
JQ

# -------------------------------------------------------------------- check
run_check() {
  jq -e '(.schema_version == "1.0") and (.workflows | type == "array") and (.repo | type == "string")' \
    "$LEDGER" >/dev/null || bad "ledger header is invalid (need schema_version 1.0, repo, workflows[])"

  # The ledger must cover exactly the workflow files on disk.
  while IFS= read -r p; do
    [ -n "$p" ] || continue
    grep -qxF "$p" "$TMP/ledger-paths" || {
      bad "$p exists but has no ledger entry — every workflow needs a recorded state"
      note "       add it with: just ci-ledger-write   (then fill in the determination fields)"
    }
  done < "$TMP/disk-paths"
  while IFS= read -r p; do
    [ -n "$p" ] || continue
    grep -qxF "$p" "$TMP/disk-paths" || bad "ledger lists $p, which does not exist — remove the entry"
  done < "$TMP/ledger-paths"

  # Per-entry validation, evaluated in jq so the rules stay in one place.
  # shellcheck disable=SC2016 # the jq program is intentionally single-quoted
  jq -r --argjson states "$STATES" --argjson dets "$DETERMINATIONS" \
        --argjson strict "$STRICT_STATES" --argjson light "$LIGHT_STATES" \
        --arg today "$TODAY" --arg horizon "$HORIZON" --arg repo "$REPO" '
    def url_ok: startswith("https://github.com/" + $repo + "/actions/runs/")
                and test("/actions/runs/[0-9]+$");
    def issue_ok: type == "string" and test("^https://github\\.com/[^/]+/[^/]+/(issues|pull)/[0-9]+$");
    def date_ok: type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}$");
    .workflows[]
    | . as $e
    | [
        (if ($e.path | test("^\\.github/workflows/[A-Za-z0-9._-]+\\.ya?ml$") | not)
           then "\($e.path): path is not a workflow path" else empty end),
        (if (($e.name // "") | length == 0) then "\($e.path): name is empty" else empty end),
        (if ($states | index($e.state) | not)
           then "\($e.path): state \"\($e.state)\" is not one of \($states | join(", "))" else empty end),
        (if ($dets | index($e.determination) | not)
           then "\($e.path): determination \"\($e.determination)\" is not one of \($dets | join(", "))" else empty end),
        (if ($e.evidence | type != "array") then "\($e.path): evidence must be an array" else empty end),
        (($e.evidence // [])[] | if (url_ok | not)
           then "\($e.path): evidence is not a \($repo) actions run URL: \(.)" else empty end),
        # green: proven by a run, and its determination may only be green|fixed
        (if ($e.state == "green" and (((($e.determination == "green") or ($e.determination == "fixed"))) | not))
           then "\($e.path): state green accepts determination green|fixed, not \"\($e.determination)\"" else empty end),
        (if ($e.state == "green" and (($e.evidence | length) == 0))
           then "\($e.path): state green needs at least one actions-run evidence URL" else empty end),
        # non-green: a written determination that cannot rot
        (if (($e.state != "green") and (($e.determination == "green") or ($e.determination == "fixed")))
           then "\($e.path): state \($e.state) needs an exempt|in-triage determination, not \"\($e.determination)\"" else empty end),
        (if (($e.state != "green") and (($e.reason // "") | length < 10))
           then "\($e.path): state \($e.state) needs a reason (at least 10 characters)" else empty end),
        (if (($e.state != "green") and ($e.review_by | date_ok | not))
           then "\($e.path): state \($e.state) needs review_by as YYYY-MM-DD" else empty end),
        (if (($e.state != "green") and ($e.review_by | date_ok) and $e.review_by < $today)
           then "\($e.path): review_by \($e.review_by) has passed — re-determine or clear it" else empty end),
        (if (($e.state != "green") and ($e.review_by | date_ok) and $e.review_by > $horizon)
           then "\($e.path): review_by \($e.review_by) is more than 90 days out" else empty end),
        # the strict tier: a red must have an owner and an issue
        (if (($strict | index($e.state)) and (($e.owner_action // "") | length < 5))
           then "\($e.path): state red needs an owner_action (the exact step that clears it)" else empty end),
        (if (($strict | index($e.state)) and (($e.issue // "") | issue_ok | not))
           then "\($e.path): state red needs an issue URL where the acceptance criteria live" else empty end),
        # the light tier: one concrete pointer is enough
        (if (($light | index($e.state)) and ((($e.owner_action // "") | length < 5) and (($e.issue // "") | issue_ok | not)))
           then "\($e.path): state \($e.state) needs an owner_action or an issue URL" else empty end),
        (if ($e.issue != null and ($e.issue | issue_ok | not))
           then "\($e.path): issue must be an issue or pull-request URL when present" else empty end),
        (if ($e.determination == "exempt" and $e.state == "green")
           then "\($e.path): exemption recorded for a green workflow — clear the exemption" else empty end)
      ]
    | .[]
  ' "$LEDGER" > "$TMP/findings"
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    bad "$f"
  done < "$TMP/findings"

  local total green determined
  total="$(jq '.workflows | length' "$LEDGER")"
  green="$(jq '[.workflows[] | select(.state == "green")] | length' "$LEDGER")"
  determined="$(jq '[.workflows[] | select(.state != "green")] | length' "$LEDGER")"
  say "ledger: $total workflow(s), $green green, $determined with a written determination"
}

# ------------------------------------------------------------------- verify
run_verify() {
  local obs
  if ! obs="$(observed_states)"; then
    warn "could not observe workflow states (${obs:-unknown reason})"
    warn "the ledger was NOT verified; offline validation (--check) still applies"
    return 0
  fi
  printf '%s' "$obs" > "$TMP/observed.json"

  jq -n --slurpfile ledger "$LEDGER" --slurpfile obs "$TMP/observed.json" "$CLASSIFY"'
    $obs[0] as $o
    | $ledger[0].workflows[]
    | . as $e
    | classify($o[$e.path]) as $seen
    | if $seen == $e.state then empty
      else {path: $e.path, recorded: $e.state, observed: $seen,
            run: ($o[$e.path].run // ""), severity: (
              if ($e.state == "green" and ($seen == "red" or $seen == "unknown"))
              then "unrecorded-red"
              elif ($e.state == "never-run") then "first-observation"
              else "bookkeeping" end)}
      end
  ' > "$TMP/drift"

  if [ -s "$TMP/drift" ]; then
    # Process substitution, not a pipe: `bad` sets FAILED, and a pipeline
    # would run it in a subshell where the flag is lost.
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      bad "state drift:$line"
    done < <(jq -r 'select(.severity == "unrecorded-red")
           | "  \(.path): recorded \(.recorded), observed \(.observed)\(if .run != "" then " — \(.run)" else "" end)"' \
      "$TMP/drift")
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      note "state drift (record it):$line"
    done < <(jq -r 'select(.severity != "unrecorded-red")
           | "  \(.path): recorded \(.recorded), observed \(.observed)"' "$TMP/drift")
    note "run: just ci-ledger-write   then record a determination for any workflow that is"
    note "no longer green, and commit the ledger in the same change."
  fi
  [ "$FAILED" -eq 0 ] && say "verify: recorded states agree with the latest runs on $BRANCH"
  return 0
}

# -------------------------------------------------------------------- write
run_write() {
  local obs
  if ! obs="$(observed_states)"; then
    printf 'ci-check-ledger: cannot write the ledger: %s\n' "${obs:-unknown reason}" >&2
    exit 2
  fi
  printf '%s' "$obs" > "$TMP/observed.json"

  jq -S -n --slurpfile ledger "$LEDGER" --slurpfile obs "$TMP/observed.json" --arg today "$TODAY" "$CLASSIFY"'
    $obs[0] as $o
    | $ledger[0]
    | .updated = $today
    | .workflows = [ .workflows[] | . as $e
        | ($o[$e.path] // null) as $run
        | (classify($run)) as $seen
        | $e
        | .state = $seen
        | .conclusion = (if $run == null then null else $run.conclusion end)
        | .observed = (if $run == null then $e.observed else $run.observed end)
        | .workflow_state = (if $run == null then $e.workflow_state else $run.workflow_state end)
        | .file_updated = (if $run == null then $e.file_updated else $run.file_updated end)
        | .evidence = (
            if $run == null or (($run.run // "") | length) == 0 then ($e.evidence // [])
            else ([$run.run] + [($e.evidence // [])[] | select(. != $run.run)]) | .[0:3]
            end
          )
      ]
  ' > "$TMP/ledger.new"

  jq -n --slurpfile old "$LEDGER" --slurpfile new "$TMP/ledger.new" '
    ($new[0].workflows | map({key: .path, value: .state}) | from_entries) as $n
    | $old[0].workflows[]
    | select($n[.path] != .state)
    | "\(.path): \(.state) -> \($n[.path])"
  ' > "$TMP/transitions"

  cp "$TMP/ledger.new" "$LEDGER"
  say "write: updated $LEDGER"
  if [ -s "$TMP/transitions" ]; then
    note "state transitions recorded:"
    while IFS= read -r t; do
      [ -n "$t" ] || continue
      note "  $t"
    done < "$TMP/transitions"
    note "any workflow that is not green needs reason, review_by and an owner_action or issue;"
    note "state red needs all four. Re-run with --check before committing."
  else
    say "write: no state changes"
  fi
}

# -------------------------------------------------------------------- drive
case "$MODE" in
  check)  run_check ;;
  verify) run_check; run_verify ;;
  write)  run_write; run_check ;;
esac

[ "$FAILED" -eq 0 ] || {
  printf '\nci-check-ledger: FAIL — see docs/audits/ci-check-ledger.json and docs/CI-RED-DETERMINATIONS.adoc.\n' >&2
  exit 1
}
exit 0
