#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Project baseline, not a claim of full RSR certification or remote settings.
set -euo pipefail
ROOT=${1:-.}
YQ_BIN=${YQ_BIN:-yq}
required=(README.adoc LICENSE LICENSES/MPL-2.0.txt LICENSES/CC-BY-SA-4.0.txt
  SECURITY.adoc SUPPORT.adoc GOVERNANCE.adoc CONTRIBUTING.adoc
  .github/SECURITY.md .github/SUPPORT.md .github/GOVERNANCE.md
  .github/CONTRIBUTING.md .github/CODE_OF_CONDUCT.md .github/CODEOWNERS
  .github/FUNDING.yml .github/pull_request_template.md
  .github/ISSUE_TEMPLATE/bug_report.yml .github/ISSUE_TEMPLATE/feature_request.yml
  .github/ISSUE_TEMPLATE/config.yml .machine_readable/rsr-profile.a2ml
  .machine_readable/descriptiles/STATE.a2ml .well-known/security.txt
  docs/STANDARDS-ALIGNMENT.adoc docs/AUDIT.adoc docs/AFFIRMATION.adoc)
for file in "${required[@]}"; do
  [[ -s "$ROOT/$file" ]] || { echo "::error::Missing/empty baseline file: $file"; exit 1; }
done
[[ ! -e "$ROOT/.github/funding.yml" ]] || { echo '::error::Duplicate funding configuration'; exit 1; }
if grep -nE '\{\{[A-Z_]+\}\}|TEMPLATE INSTRUCTIONS' "$ROOT"/.github/{CODE_OF_CONDUCT,SECURITY,SUPPORT,GOVERNANCE}.md "$ROOT"/.github/ISSUE_TEMPLATE/*; then
  echo '::error::Uninstantiated community template'; exit 1
fi
for form in bug_report feature_request; do
  "$YQ_BIN" -o=json '.' "$ROOT/.github/ISSUE_TEMPLATE/$form.yml" | jq -e '
    (.name | type == "string") and (.description | type == "string") and
    (.body | type == "array" and length > 0) and
    ([.body[].id? | select(. != null)] | length == (unique | length)) and
    any(.body[]; .validations.required == true)' >/dev/null
done
"$YQ_BIN" -o=json '.' "$ROOT/.github/ISSUE_TEMPLATE/config.yml" | jq -e '
  any(.contact_links[]; .url == "https://github.com/hyperpolymath/rpa-elysium/security/advisories/new")' >/dev/null
grep -q 'security/advisories/new' "$ROOT/.github/SECURITY.md"
grep -q 'blob/main/SECURITY.adoc' "$ROOT/.well-known/security.txt"
expires=$(sed -n 's/^Expires: //p' "$ROOT/.well-known/security.txt")
[[ $(date -u -d "$expires" +%s) -gt $(date -u +%s) ]] || { echo '::error::security.txt expired'; exit 1; }
for target in SECURITY SUPPORT GOVERNANCE; do
  grep -q "../$target.adoc" "$ROOT/.github/$target.md"
done
echo 'Repository health: local baseline, instantiated forms, policy pointers and security.txt valid.'
echo 'Remote feature/ruleset state and full canonical-layout conformity require the separate alignment audit.'
