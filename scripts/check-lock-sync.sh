#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# check-lock-sync.sh — verify .github/workflows/actions.lock is in sync with the
# workflow YAML, in BOTH directions (including job-level reusable-workflow refs),
# AND that the lockfile is TRANSITIVELY CLOSED.
#
# Three clauses, each of which alone is insufficient:
#
#   1. every `uses:` in a workflow is locked under THAT workflow's own path;
#   2. every lockfile entry is still referenced by its workflow (no orphans);
#   3. every ref NAMED anywhere in the lockfile resolves to a top-level
#      `dependencies:` record — the lockfile has no dangling edges.
#
# Clause 3 is not decoration. It is the clause that catches the failure mode that
# clauses 1 and 2 are structurally blind to, and it was added only after that
# blindness was measured. On hyperpolymath/cicd-squabbler, 2026-09-22:
#
#   commit    dangling-edge class                              result
#   fe22bbc   workflows: -> dependencies: (ref listed, no record)   4 workflows startup_failure, jobs=0
#   cfadcf9   dependencies: -> dependencies: (record added, its
#             own nested uses: unrecorded)                          the same 4 still startup_failure
#   5286aa5   none - transitively closed                            0 startup_failure, all 17 runs create jobs
#
# At fe22bbc AND cfadcf9 this script exited 0, `gh actions-lock --verify-local`
# exited 0, and the Lock Sync Gate reported green - while GitHub was refusing to
# start four workflows. Every local gate was green on a fatal commit. That is the
# guard/consumer trap: the gate asked "is every uses: locked?" and GitHub asks
# "is every locked ref RESOLVABLE?".
#
# The asymmetry that makes clause 3 mandatory, and counter-intuitive:
#   * a job-level ref ABSENT from the lockfile entirely is HARMLESS;
#   * a ref PRESENT in the lockfile but unresolvable is FATAL.
# So adding entries without closing them is strictly worse than adding nothing.
# Clause 1 demands entries be added; only clause 3 makes that demand safe. Shipping
# clause 1 without clause 3 actively steers a developer into the fatal state:
# Dependabot bumps a job-level ref -> clause 1 reds -> `gh actions-lock` is blind to
# job-level refs and will not backfill -> the developer hand-adds the workflows:
# entry to get green -> no dependencies: record -> CI dies silently, gate green.
#
# Exit 0 only when all three clauses hold. Any violation exits 1. There is no
# warn-only mode: a desync means GitHub refuses to start the run, so it must fail
# the job. A `::warning::` cannot fail a job and would be a vacuous gate.

set -euo pipefail

WF_DIR="${1:-.github/workflows}"
LOCK="$WF_DIR/actions.lock"

# gawk is required: the parser uses 3-argument match(), a GNU extension. mawk
# (the Debian/Ubuntu default `awk`) does not support it, and a silent parse
# failure here would read as a clean pass - the exact failure mode this script
# exists to prevent. Probe it rather than trusting the name.
AWK=""
for cand in gawk awk; do
  if command -v "$cand" >/dev/null 2>&1 \
     && echo x | "$cand" '{ if (match($0, /(x)/, m) && m[1] == "x") exit 0; exit 1 }' 2>/dev/null; then
    AWK="$cand"; break
  fi
done
if [ -z "$AWK" ]; then
  echo "check-lock-sync: FATAL: no awk supporting 3-argument match() (need gawk)" >&2
  echo "check-lock-sync: install it with: sudo apt-get install -y gawk" >&2
  exit 1
fi

if [ ! -f "$LOCK" ]; then
  echo "check-lock-sync: FATAL: no lockfile at $LOCK" >&2
  exit 1
fi

shopt -s nullglob
mapfile -t WORKFLOWS < <(printf '%s\n' "$WF_DIR"/*.yml "$WF_DIR"/*.yaml | sort -u)
if [ "${#WORKFLOWS[@]}" -eq 0 ]; then
  echo "check-lock-sync: FATAL: no workflow files under $WF_DIR" >&2
  exit 1
fi

read -r -d '' PROG <<'AWK' || true
# Normalise an external action or reusable-workflow reference to owner/repo@ref.
# Drop any subpath and return an empty string for local or malformed references.
function norm(r,   at, path, ref, n, parts) {
  at = 0
  for (n = length(r); n > 0; n--) { if (substr(r, n, 1) == "@") { at = n; break } }
  if (at == 0) return ""
  path = substr(r, 1, at - 1); ref = substr(r, at + 1)
  if (path == "" || ref == "") return ""
  if (substr(path, 1, 2) == "./" || substr(path, 1, 2) == "$/") return ""   # local action
  if (split(path, parts, "/") < 2) return ""
  return parts[1] "/" parts[2] "@" ref
}

# Fold case on the OWNER/REPO segment only, for comparison keys. GitHub resolves
# owner and repository names case-insensitively, and this is measured, not assumed:
# metadatastician/pong-ping's lockfile records sonarsource/sonarqube-scan-action@v8.2.1
# while sonarqube.yml says SonarSource/..., and at commit cd5f90f that workflow ran
# SUCCESS while codeql.yml at the SAME commit was startup_failure. A same-commit
# control, so the case difference is provably not what kills a run.
# The REF is NOT folded: git tags and branch names are case-sensitive.
function ck(r,   at, s) {
  at = 0
  for (s = length(r); s > 0; s--) { if (substr(r, s, 1) == "@") { at = s; break } }
  if (at == 0) return tolower(r)
  return tolower(substr(r, 1, at - 1)) substr(r, at)
}

# ---------- pass 1: the lockfile ----------
FILENAME == lockfile {
  if ($0 ~ /^workflows:[[:space:]]*$/)    { inwf = 1; indep = 0; next }
  if ($0 ~ /^dependencies:[[:space:]]*$/) { inwf = 0; indep = 1; next }
  if ($0 ~ /^[a-z_]+:/)                   { inwf = 0; indep = 0; next }

  # --- the dependencies: section, for clause 3 ---
  if (indep) {
    # "    'owner/repo@ref':"  -- a top-level dependency record
    if (match($0, /^    '([^']+)':/, m)) {
      depkey = m[1]
      haverec[ck(depkey)] = 1; disp[ck(depkey)] = depkey
      next
    }
    # "            - 'owner/repo@ref'"  -- a nested uses: of that record
    if (match($0, /^            - '([^']+)'/, m) && depkey != "") {
      r = ck(m[1]); disp[r] = m[1]
      want[r] = 1
      wantsrc[r] = wantsrc[r] " dependencies:" depkey
      next
    }
    next
  }

  if (!inwf) next

  # "    '.github/workflows/x.yml':"  or  "... : []"
  if (match($0, /^    '([^']+)':/, m)) {
    cur = m[1]
    seen_path[cur] = 1
    next
  }
  if (match($0, /^        - '([^']+)'[[:space:]]*$/, m) && cur != "") {
    lr = ck(m[1]); disp[lr] = m[1]; lock[cur, lr] = 1
    lockcount[cur]++
    want[lr] = 1
    wantsrc[lr] = wantsrc[lr] " " cur
    next
  }
  next
}

# ---------- pass 2: the workflow YAML ----------
FNR == 1 { wf = FILENAME }
{
  line = $0
  sub(/[[:space:]]+#.*$/, "", line)              # strip trailing comment
  if (match(line, /^[[:space:]]*-?[[:space:]]*uses:[[:space:]]*(.+)$/, m)) {
    raw = m[1]
    gsub(/^["']|["']$/, "", raw)
    gsub(/[[:space:]]+$/, "", raw)
    n = norm(raw)
    if (n != "") { uses[wf, ck(n)] = 1; useslist[wf] = useslist[wf] " " n }
  }
}

END {
  bad = 0
  for (i = 1; i < ARGC; i++) {
    wf = ARGV[i]
    if (wf == lockfile) continue
    key = wf
    sub(/.*\//, "", key)
    key = ".github/workflows/" key          # the lockfile always uses this canonical path

    # --- clause 1: every uses: must be locked under THIS path ---
    nu = split(useslist[wf], u, " ")
    delete uniq; missing = ""
    for (j = 1; j <= nu; j++) {
      if (u[j] == "" || (u[j] in uniq)) continue
      uniq[u[j]] = 1
      if (!((key SUBSEP ck(u[j])) in lock)) missing = missing " " u[j]
    }
    if (missing != "") {
      if (!(key in seen_path))
        printf "FAIL %s\n     not onboarded: no lockfile entry for this path\n     unlocked refs:%s\n", key, missing
      else
        printf "FAIL %s\n     refs missing from the lockfile:%s\n", key, missing
      bad = 1
    }

    # --- clause 2: every lock entry must be referenced by this workflow ---
    orphan = ""
    for (k in lock) {
      split(k, kp, SUBSEP)
      if (kp[1] != key) continue
      if (!((wf SUBSEP kp[2]) in uses)) orphan = orphan " " (kp[2] in disp ? disp[kp[2]] : kp[2])
    }
    if (orphan != "") {
      printf "FAIL %s\n     stale lockfile entries, no uses: references them:%s\n", key, orphan
      bad = 1
    }
  }

  # --- lockfile entries for workflow files that no longer exist ---
  for (p in seen_path) {
    found = 0
    for (i = 1; i < ARGC; i++) {
      q = ARGV[i]; if (q == lockfile) continue
      sub(/.*\//, "", q); q = ".github/workflows/" q
      if (q == p) { found = 1; break }
    }
    if (!found) { printf "FAIL %s\n     lockfile entry for a workflow file that does not exist\n", p; bad = 1 }
  }

  # --- clause 3: TRANSITIVE CLOSURE. Every ref named anywhere in the lockfile
  #     must resolve to a top-level dependencies: record. A dangling edge makes
  #     GitHub refuse the run at startup with jobs=0. ---
  ndang = 0; dang = ""
  for (r in want) {
    if (r !~ /^[^\/]+\/[^\/@]+@/) continue      # not an OWNER/REPO@REF pin; not ours to resolve
    if (r in haverec) continue
    ndang++
    dang = dang sprintf("\n       %s\n           named by:%s", (r in disp ? disp[r] : r), wantsrc[r])
  }
  if (ndang > 0) {
    printf "FAIL actions.lock: DANGLING EDGES\n"
    printf "     %d ref(s) are named in the lockfile but have no top-level dependencies: record.%s\n", ndang, dang
    bad = 1
  }

  # --- a dependencies: record nothing names is dead weight, not fatal: report only ---
  nunref = 0
  for (d in haverec) if (!(d in want)) nunref++

  if (bad) {
    print ""
    print "actions.lock is OUT OF SYNC with the workflow YAML, or is not transitively closed."
    print "GitHub refuses such a run at startup: zero jobs are created and the run"
    print "reports \"This run likely failed because of a workflow file issue.\""
    print ""
    print "Fix, in this order:"
    print "  1. `gh actions-lock --no-migrate-local-actions`, then review the diff. It does"
    print "     NOT handle job-level reusable-workflow refs and it can de-pin bare SHAs to"
    print "     floating tags - both must be corrected by hand."
    print "  2. For any DANGLING EDGES above, add a top-level `dependencies:` record for each"
    print "     ref. A leaf record may legally omit the nested `uses:` key entirely, so adding"
    print "     leaves introduces no new dangling edges and closure terminates in one pass."
    print "     Keys are sorted with LC_ALL=C collation (ASCII '-' 0x2d sorts before '@' 0x40)."
    print "  3. Nested `uses:` entries must be bare OWNER/REPO@REF. A subpath pin such as"
    print "     github/codeql-action/upload-sarif@<sha> is REJECTED by the schema; collapse it"
    print "     to github/codeql-action@<sha>."
    exit 1
  }
  printf "actions.lock is in sync and transitively closed:\n"
  printf "  * every uses: is locked under its own workflow path (job-level reusable refs included)\n"
  printf "  * every lockfile entry is still referenced\n"
  printf "  * every ref named in the lockfile resolves to a dependencies: record (0 dangling edges)\n"
  if (nunref > 0)
    printf "  note: %d dependencies: record(s) are unreferenced - harmless, but prunable.\n", nunref
}
AWK

"$AWK" -v lockfile="$LOCK" "$PROG" "$LOCK" "${WORKFLOWS[@]}"
