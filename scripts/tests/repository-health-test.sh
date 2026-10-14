#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail
PROJECT="$(cd "$(dirname "$0")/../.." && pwd)"
CHECK="$PROJECT/scripts/check-repository-health.sh"
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
# Test fixtures preserve symlink targets without copying source/build artefacts.
cp -R "$PROJECT/.github" "$PROJECT/LICENSES" "$PROJECT/.machine_readable" "$PROJECT/www" "$PROJECT/docs" "$scratch/"
cp "$PROJECT/README.adoc" "$PROJECT/LICENSE" "$PROJECT/SECURITY.adoc" "$PROJECT/SUPPORT.adoc" "$PROJECT/GOVERNANCE.adoc" "$PROJECT/CONTRIBUTING.adoc" "$scratch/"
ln -s www/.well-known "$scratch/.well-known"
bash "$CHECK" "$scratch"
echo 'PASS: complete local community baseline'
expect_failure() {
  if bash "$CHECK" "$scratch" > "$scratch/output" 2>&1; then
    echo "FAIL: $1 was accepted"; exit 1
  fi
  echo "PASS: $1 fails closed"
}
mv "$scratch/.github/SECURITY.md" "$scratch/.github/SECURITY.saved"
expect_failure 'missing private-security surface'
mv "$scratch/.github/SECURITY.saved" "$scratch/.github/SECURITY.md"
echo '{{SECURITY_EMAIL}}' >> "$scratch/.github/SECURITY.md"
expect_failure 'uninstantiated template token'
cp "$PROJECT/.github/SECURITY.md" "$scratch/.github/SECURITY.md"
sed -i 's/Expires:.*/Expires: 2000-01-01T00:00:00Z/' "$scratch/www/.well-known/security.txt"
expect_failure 'expired security.txt'
cp "$PROJECT/www/.well-known/security.txt" "$scratch/www/.well-known/security.txt"
cp "$scratch/.github/FUNDING.yml" "$scratch/.github/funding.yml"
expect_failure 'duplicate funding configuration'
rm "$scratch/.github/funding.yml"
bash "$CHECK" "$scratch"
echo '5 repository-health controls passed.'
