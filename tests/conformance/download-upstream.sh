#!/usr/bin/env bash
# Download upstream React Compiler fixture inputs from facebook/react.
#
# Usage:
#   ./tests/conformance/download-upstream.sh
#
# This downloads the fixture input files from the upstream React Compiler
# test suite into tests/conformance/upstream-fixtures/. The script uses
# the GitHub API to list files and wget/curl to download them.
#
# Prerequisites: curl, jq (for JSON parsing)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UPSTREAM_DIR="$SCRIPT_DIR/upstream-fixtures"
REPO="facebook/react"
BRANCH="main"
FIXTURE_PATH="compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler"

echo "Downloading upstream React Compiler fixtures..."
echo "Repository: $REPO (branch: $BRANCH)"
echo "Path: $FIXTURE_PATH"
echo "Output: $UPSTREAM_DIR"
echo ""

# Create output directory
mkdir -p "$UPSTREAM_DIR"

# Use GitHub API to list files in the fixture directory (recursive).
# The tree API can list up to 100,000 files.
API_URL="https://api.github.com/repos/$REPO/git/trees/$BRANCH?recursive=1"

echo "Fetching file tree from GitHub API..."
TREE_JSON=$(curl -sS -H "Accept: application/vnd.github.v3+json" "$API_URL")

# Extract paths matching the fixture directory
echo "$TREE_JSON" | jq -r ".tree[] | select(.path | startswith(\"$FIXTURE_PATH/\")) | select(.type == \"blob\") | .path" > /tmp/upstream-fixtures-list.txt

TOTAL=$(wc -l < /tmp/upstream-fixtures-list.txt)
echo "Found $TOTAL fixture files."
echo ""

# Download each file
COUNT=0
while IFS= read -r filepath; do
    # Compute relative path within the fixtures directory
    REL_PATH="${filepath#$FIXTURE_PATH/}"
    OUTPUT_FILE="$UPSTREAM_DIR/$REL_PATH"
    OUTPUT_DIR="$(dirname "$OUTPUT_FILE")"

    # Only download input files (skip __snapshots__ and other test artifacts)
    case "$REL_PATH" in
        *__snapshots__*) continue ;;
        *.snap) continue ;;
        *.expected*) continue ;;
    esac

    mkdir -p "$OUTPUT_DIR"

    # Download raw file content
    RAW_URL="https://raw.githubusercontent.com/$REPO/$BRANCH/$filepath"
    curl -sS -o "$OUTPUT_FILE" "$RAW_URL"

    COUNT=$((COUNT + 1))
    if [ $((COUNT % 50)) -eq 0 ]; then
        echo "  Downloaded $COUNT / $TOTAL files..."
    fi
done < /tmp/upstream-fixtures-list.txt

echo ""
echo "Done! Downloaded $COUNT fixture files to $UPSTREAM_DIR"
rm -f /tmp/upstream-fixtures-list.txt
