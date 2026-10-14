#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# reconcile-audit-issue.sh — keep exactly one dependency-vulnerability issue,
# with the findings in it, and close it with evidence when the audit passes.
#
# Why this exists: the previous version of .github/workflows/cargo-audit.yml
# created an issue whose whole body was "Run `cargo audit` locally for details"
# and never closed it. rpa-elysium#143 and rpa-elysium#113 are that issue twice,
# with no findings, no acceptance criteria, and no way to tell from the issue
# itself whether it was already fixed. An issue nobody can act on is noise that
# hides real reds.
#
# Behaviour:
#   audit failed  -> adopt or create ONE issue carrying a stable marker, the
#                    cargo-audit output, the run that produced it, and
#                    acceptance criteria. A new failing run is commented, so
#                    the history of findings survives.
#   audit passed  -> comment the green run and close the marked issue. Closing
#                    is evidence, not muting: the gate that opened it proves it
#                    fixed on a run of the default branch.
#   pull_request  -> never touched. Issues are reconciled from the default
#                    branch and scheduled runs only.
#
# Usage:
#   scripts/reconcile-audit-issue.sh --status N --run URL [--summary FILE]
#                                   [--repo OWNER/NAME] [--event NAME]
#                                   [--marker TEXT] [--title TEXT]
#                                   [--label security] [--label dependencies]
#                                   [--dry-run]
# Exit: 0 reconciled | 1 the reconciliation failed | 2 setup failure

set -uo pipefail

STATUS=""
RUN_URL=""
SUMMARY=""
REPO="${GITHUB_REPOSITORY:-hyperpolymath/rpa-elysium}"
EVENT="${GITHUB_EVENT_NAME:-push}"
MARKER="<!-- rpa-elysium:cargo-audit -->"
TITLE="Security: Dependency vulnerabilities detected"
LABELS=()
DRY_RUN=0
FAILED=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --status)  STATUS="$2"; shift ;;
    --run)     RUN_URL="$2"; shift ;;
    --summary) SUMMARY="$2"; shift ;;
    --repo)    REPO="$2"; shift ;;
    --event)   EVENT="$2"; shift ;;
    --marker)  MARKER="$2"; shift ;;
    --title)   TITLE="$2"; shift ;;
    --label)   LABELS+=("$2"); shift ;;
    --dry-run) DRY_RUN=1 ;;
    -h|--help) sed -n '2,30p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) printf 'reconcile-audit-issue: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

say() { printf 'reconcile-audit-issue: %s\n' "$*"; }
bad() { printf 'reconcile-audit-issue: FAIL: %s\n' "$*" >&2; FAILED=1; }

command -v jq >/dev/null 2>&1 || { printf 'reconcile-audit-issue: jq is required\n' >&2; exit 2; }
[ -n "$STATUS" ] || { printf 'reconcile-audit-issue: --status is required\n' >&2; exit 2; }
[ -n "$RUN_URL" ] || { printf 'reconcile-audit-issue: --run is required\n' >&2; exit 2; }
if [ "${#LABELS[@]}" -eq 0 ]; then LABELS=(security dependencies); fi

if [ "$EVENT" = "pull_request" ]; then
  say "pull_request event: issues are reconciled from default-branch and scheduled runs only"
  exit 0
fi

if [ "$DRY_RUN" -eq 0 ]; then
  command -v gh >/dev/null 2>&1 || { printf 'reconcile-audit-issue: gh is required\n' >&2; exit 2; }
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

gh_issue() {
  if [ "$DRY_RUN" -eq 1 ]; then
    printf 'reconcile-audit-issue: would run: gh issue %s\n' "$*"
    return 0
  fi
  gh issue "$@" || bad "gh issue $1 failed"
}

# ------------------------------------------------------------- find the issue
label_args=()
for label in "${LABELS[@]}"; do label_args+=(--label "$label"); done

if [ "$DRY_RUN" -eq 0 ]; then
  if ! gh issue list --repo "$REPO" --state open "${label_args[@]}" --limit 100 \
       --json number,title,body > "$TMP/open.json" 2> "$TMP/open.err"; then
    printf 'reconcile-audit-issue: cannot list issues: %s\n' "$(head -1 "$TMP/open.err")" >&2
    exit 1
  fi
else
  printf '%s' '[]' > "$TMP/open.json"
fi

number="$(jq -r --arg m "$MARKER" '[.[] | select(.body | contains($m))][0].number // ""' "$TMP/open.json")"
adopted=""
if [ -z "$number" ]; then
  # Adopt an issue created by the previous, markerless publisher: same title,
  # same labels, no findings. Otherwise the old issue would never close.
  number="$(jq -r --arg t "$TITLE" '[.[] | select(.title == $t)][0].number // ""' "$TMP/open.json")"
  [ -n "$number" ] && adopted=yes
fi

# ------------------------------------------------------------- body assembly
findings=""
if [ -n "$SUMMARY" ] && [ -f "$SUMMARY" ]; then
  findings="$(tail -c 6000 "$SUMMARY")"
else
  findings="(no cargo-audit output was captured — the audit failed before it produced any)"
fi

write_body() { # failing | passing
  local kind="$1"
  {
    printf '%s\n' "$MARKER"
    printf '<!-- last-run: %s -->\n\n' "$RUN_URL"
    if [ "$kind" = failing ]; then
      printf '## Summary\n\n'
      printf '`cargo audit --deny warnings` / `--deny unmaintained` failed on `%s`.\n\n' "$REPO"
      printf 'Run: %s\n\n' "$RUN_URL"
      printf '## Findings\n\n<details><summary>cargo audit output</summary>\n\n```\n%s\n```\n\n</details>\n\n' "$findings"
      cat <<'BODY'
## Acceptance criteria

- [ ] Every advisory above is addressed: upgrade to a release that fixes it, or
      remove or replace the dependency.
- [ ] If no fixed release exists upstream and the code path is not reachable,
      a documented waiver is added in the same change that needs it — the
      advisory ID, why it cannot be fixed here, and a review date.
- [ ] `cargo audit --deny warnings` exits 0 on a run of the default branch.
      This issue closes itself with that run linked, so "verified fixed" is
      evidence rather than an assertion.
BODY
      printf '\nAutomation: this issue is maintained by `.github/workflows/cargo-audit.yml`\n'
      printf 'through `scripts/reconcile-audit-issue.sh`. It carries the marker above so\n'
      printf 'the workflow can find it again, and it is closed only after the audit passes.\n'
    else
      printf '## Resolved\n\n'
      printf '`cargo audit --deny warnings` and `cargo audit --deny unmaintained` both\n'
      printf 'exited 0 on a run of the default branch: %s\n\n' "$RUN_URL"
      printf 'The gate that opened this issue closed it, so the fix is verified rather\n'
      printf 'than asserted. If the audit fails again the workflow reopens the record by\n'
      printf 'creating or updating this issue with the new findings.\n'
    fi
  } > "$TMP/body.md"
}

if [ "$STATUS" -eq 0 ]; then
  # ------------------------------------------------------------- green path
  if [ -z "$number" ]; then
    say "audit passed and no open vulnerability issue exists: nothing to reconcile"
    exit 0
  fi
  write_body passing
  say "audit passed: closing #$number with $RUN_URL"
  # The body is rewritten as well, so a future failure finds this issue by its
  # marker rather than by its title, and the record of why it closed lives in
  # the issue rather than only in a comment.
  gh_issue edit "$number" --repo "$REPO" --body-file "$TMP/body.md"
  gh_issue close "$number" --repo "$REPO" \
    --comment "Verified fixed: $RUN_URL — \`cargo audit --deny warnings\` and \`cargo audit --deny unmaintained\` both exited 0 on a run of the default branch."
  [ "$FAILED" -eq 0 ] || exit 1
  say "closed #$number"
  exit 0
fi

# ------------------------------------------------------------ failing path
write_body failing
if [ -z "$number" ]; then
  say "audit failed: opening one vulnerability issue"
  gh_issue create --repo "$REPO" --title "$TITLE" "${label_args[@]}" --body-file "$TMP/body.md"
else
  if [ "$adopted" = yes ]; then
    say "adopting markerless issue #$number created by the previous publisher"
  fi
  last_run="$(jq -r --arg m "$MARKER" '[.[] | select(.body | contains($m))][0].body // ""' "$TMP/open.json" \
             | sed -n 's/.*<!-- last-run: \(.*\) -->.*/\1/p' | head -1)"
  if [ "$last_run" = "$RUN_URL" ]; then
    say "audit failed: #$number already records this run; updating the report only"
    gh_issue edit "$number" --repo "$REPO" --body-file "$TMP/body.md"
  else
    say "audit failed: updating #$number with the new findings"
    gh_issue edit "$number" --repo "$REPO" --body-file "$TMP/body.md"
    gh_issue comment "$number" --repo "$REPO" --body "New failing audit run: $RUN_URL"
  fi
fi

[ "$FAILED" -eq 0 ] || exit 1
say "reconciled"
exit 0
