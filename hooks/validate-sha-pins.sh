#!/usr/bin/env bash
# SPDX-License-Identifier: PMPL-1.0-or-later
# Pre-commit hook: Validate GitHub Actions are SHA-pinned
#
# Only checks actual YAML-level `uses:` directives.
# Ignores `uses:` strings inside run: shell blocks.

set -euo pipefail

ERRORS=0

for workflow in .github/workflows/*.yml .github/workflows/*.yaml; do
    [ -f "$workflow" ] || continue

    # Use yq-style approach: only match lines that are YAML step uses: keys
    # These are indented with spaces and followed by owner/repo@ref format
    while IFS= read -r line; do
        # Match YAML uses: directives (leading whitespace + uses: + owner/repo pattern)
        # Skip lines inside run: blocks (those contain shell code with "uses:" as strings)
        if echo "$line" | grep -qP '^\s+uses:\s+[a-zA-Z0-9_-]+/[a-zA-Z0-9_.-]+@'; then
            # Check if it has a SHA (40 hex chars after @)
            if ! echo "$line" | grep -qE '@[a-f0-9]{40}'; then
                echo "ERROR: Unpinned action in $workflow"
                echo "  $line"
                echo "  Actions must use SHA pins: uses: action/name@SHA # version"
                ERRORS=$((ERRORS + 1))
            fi
        fi
    done < "$workflow"
done

if [ $ERRORS -gt 0 ]; then
    echo ""
    echo "Found $ERRORS unpinned actions. Please SHA-pin all GitHub Actions."
    exit 1
fi

exit 0
