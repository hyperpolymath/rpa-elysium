#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# check-mirror-config.sh — the enabled mirror set must match the declared one,
# and every enabled forge must actually be configured.
#
# This reads repository *variables* only. Key secrets are deliberately not read:
# an empty key is already handled by the reusable (the job prints a skip notice
# rather than mirroring), and copying SSH keys into a job that only compares sets
# would buy nothing. What this catches is the variable-shaped failure the keys
# cannot explain — an empty vars.GITEA_HOST, and forges enabled without being
# declared.
#
# Why this exists: `mirror.yml` delegates to a reusable that fans out to one job
# per forge, each gated on a repository variable (`<FORGE>_MIRROR_ENABLED`) and
# skipping itself when its key is absent. When a *variable* is missing instead
# of a secret — the gitea job reads `vars.GITEA_HOST` — the job still starts and
# dies inside `ssh-keyscan`, leaving `main` red with an exit code and nothing
# else to go on. Both failures this repo hit on 2026-09-15 were of that shape:
# `mirror-gitea` exit 1, `mirror-disroot` exit 128.
#
# The declared set lives in .machine_readable/6a2/META.a2ml (`forges = [...]`),
# which is also what README.adoc §Multi-Forge Distribution and ROADMAP.adoc
# state. Enabling a forge there without declaring it — or declaring one that is
# not enabled — is drift, and drift is what this script fails on.
#
# Usage: scripts/check-mirror-config.sh [REPO_ROOT]
# Exit:  0 consistent | 1 drift or unconfigured enabled forge | 2 setup failure

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/.."
if [ "$#" -gt 0 ] && [ -d "${1:-}" ] && [ "${1#-}" = "$1" ]; then
  REPO_ROOT="$1"
  shift
fi
REPO_ROOT="$(cd "$REPO_ROOT" && pwd)"
case "$#" in
  0) ;;
  -h|--help) sed -n '2,22p' "${BASH_SOURCE[0]}"; exit 0 ;;
  *) printf 'check-mirror-config: unknown argument: %s\n' "$1" >&2; exit 2 ;;
esac

META="$REPO_ROOT/.machine_readable/6a2/META.a2ml"
[ -f "$META" ] || { printf 'check-mirror-config: %s not found\n' "$META" >&2; exit 2; }

FAILED=0
forges=(codeberg gitlab bitbucket sourcehut radicle gitea disroot)

enabled_flag() { # forge -> true|false|"" (unset), by indirect expansion
  local upper name
  upper="$(printf '%s' "$1" | tr '[:lower:]' '[:upper:]')"
  name="${upper}_MIRROR_ENABLED"
  printf '%s' "${!name-}"
}

host_value() { # forge -> the host the reusable targets
  case "$1" in
    gitea) printf '%s' "${GITEA_HOST:-}" ;;
    codeberg) printf 'codeberg.org' ;;
    gitlab) printf 'gitlab.com' ;;
    bitbucket) printf 'bitbucket.org' ;;
    sourcehut) printf 'git.sr.ht' ;;
    disroot) printf 'git.disroot.org' ;;
    radicle) printf 'p2p seed node' ;;
  esac
}

# Declared mirrors: the META.a2ml forges minus the origin itself.
declared="$(sed -n 's/^forges *= *\[\(.*\)\]$/\1/p' "$META" | tr -d '"' | tr ',' '\n' \
            | sed 's/^[[:space:]]*//; s/[[:space:]]*$//' | grep -v '^github$' | grep -v '^$' || true)"
[ -n "$declared" ] || { printf 'check-mirror-config: no forges list found in %s\n' "$META" >&2; exit 2; }

printf 'check-mirror-config: declared mirrors: %s\n' "$(printf '%s' "$declared" | tr '\n' ' ')"
printf '%-10s %-8s %-9s %-14s %s\n' forge enabled declared host verdict
for forge in "${forges[@]}"; do
  flag="$(enabled_flag "$forge")"
  host="$(host_value "$forge")"
  if printf '%s\n' "$declared" | grep -qx "$forge"; then shown_declared=yes; else shown_declared=no; fi
  if [ "$flag" = "true" ]; then shown_enabled=yes; else shown_enabled=no; fi
  if [ -n "$host" ]; then shown_host="$host"; else shown_host="unset"; fi

  verdict="ok"
  if [ "$shown_enabled" = yes ] && [ "$shown_declared" = no ]; then
    verdict=undeclared
    printf '::error::check-mirror-config: the %s mirror is enabled for this repository but %s does not declare it (README.adoc and ROADMAP.adoc agree with META.a2ml). Declare it or disable it: gh variable delete %s_MIRROR_ENABLED\n' \
      "$forge" ".machine_readable/6a2/META.a2ml" "$(printf '%s' "$forge" | tr '[:lower:]' '[:upper:]')"
    FAILED=1
  fi
  if [ "$shown_enabled" = yes ] && [ "$forge" = gitea ] && [ -z "$host" ]; then
    verdict=no-host
    printf '::error::check-mirror-config: the gitea mirror is enabled but vars.GITEA_HOST is empty, so the ssh-keyscan step fails with exit 1 before any push is attempted. Set it (gh variable set GITEA_HOST --body <host>) or delete GITEA_MIRROR_ENABLED.\n'
    FAILED=1
  fi
  if [ "$shown_enabled" = no ] && [ "$shown_declared" = yes ]; then
    if [ "$verdict" = ok ]; then verdict="declared-off"; fi
    printf '::warning::check-mirror-config: %s is declared in META.a2ml but not enabled, so the declaration overstates what happens: gh variable set %s_MIRROR_ENABLED --body true, or remove it from the declaration.\n' \
      "$forge" "$(printf '%s' "$forge" | tr '[:lower:]' '[:upper:]')"
  fi
  printf '%-10s %-8s %-9s %-14s %s\n' "$forge" "$shown_enabled" "$shown_declared" "$shown_host" "$verdict"
done

if [ "$FAILED" -ne 0 ]; then
  printf '\ncheck-mirror-config: FAIL — the enabled mirror set and the declared mirror set disagree.\n' >&2
  printf 'See docs/CI-RED-DETERMINATIONS.adoc and hyperpolymath/rpa-elysium#147.\n' >&2
  exit 1
fi
printf 'check-mirror-config: OK — every enabled mirror is declared and configured.\n'
