#!/usr/bin/env bash
#
# Extract benchmark fixtures from OSS React repositories.
# Each repo is cloned at a pinned commit and selected component files are copied.
#
# Usage: ./extract-fixtures.sh [--clean]
#   --clean: Remove existing extracted fixtures before re-extracting

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../fixtures"
TMP_DIR="$(mktemp -d)"

trap 'rm -rf "$TMP_DIR"' EXIT

# --- Repository definitions ---
# Format: repo_url commit_sha target_dir files...
declare -A REPOS
declare -A COMMITS
declare -A FILES

# cal.com - scheduling app, heavy hook usage
REPOS[calcom]="https://github.com/calcom/cal.com"
COMMITS[calcom]="main"  # Pin to a specific commit before production use
FILES[calcom]="
apps/web/components/booking/BookingListItem.tsx
apps/web/components/availability/Schedule.tsx
packages/ui/components/form/DatePicker.tsx
"

# excalidraw - canvas app, useEffect-heavy
REPOS[excalidraw]="https://github.com/excalidraw/excalidraw"
COMMITS[excalidraw]="master"
FILES[excalidraw]="
packages/excalidraw/components/LayerUI.tsx
packages/excalidraw/components/Sidebar/Sidebar.tsx
packages/excalidraw/components/ColorPicker/ColorPicker.tsx
"

# shadcn/ui - component library, clean patterns
REPOS[shadcnui]="https://github.com/shadcn-ui/ui"
COMMITS[shadcnui]="main"
FILES[shadcnui]="
apps/www/components/main-nav.tsx
apps/www/registry/default/ui/command.tsx
apps/www/registry/default/ui/data-table.tsx
"

if [[ "${1:-}" == "--clean" ]]; then
  echo "Cleaning extracted fixtures..."
  for repo in "${!REPOS[@]}"; do
    rm -rf "$FIXTURES_DIR/$repo"
  done
fi

extract_repo() {
  local name="$1"
  local url="${REPOS[$name]}"
  local commit="${COMMITS[$name]}"
  local files="${FILES[$name]}"
  local clone_dir="$TMP_DIR/$name"

  echo "=== Extracting $name from $url @ $commit ==="

  # Shallow clone at the specified commit
  git clone --depth 1 --branch "$commit" "$url" "$clone_dir" 2>/dev/null || {
    echo "  WARN: Could not clone $name, skipping"
    return 0
  }

  local actual_commit
  actual_commit=$(git -C "$clone_dir" rev-parse HEAD)
  echo "  Commit: $actual_commit"

  mkdir -p "$FIXTURES_DIR/$name"

  local count=0
  for file in $files; do
    file=$(echo "$file" | xargs)  # trim whitespace
    [[ -z "$file" ]] && continue

    local src="$clone_dir/$file"
    if [[ -f "$src" ]]; then
      local dest_name
      dest_name=$(basename "$file")
      cp "$src" "$FIXTURES_DIR/$name/$dest_name"
      echo "  Copied: $file -> $name/$dest_name"
      ((count++))
    else
      echo "  SKIP: $file (not found)"
    fi
  done

  echo "  Extracted $count files from $name"

  # Record provenance
  echo "{\"repo\": \"$url\", \"commit\": \"$actual_commit\", \"extracted\": \"$(date -Iseconds)\", \"files\": $count}" \
    > "$FIXTURES_DIR/$name/provenance.json"
}

for repo in "${!REPOS[@]}"; do
  extract_repo "$repo"
done

echo ""
echo "Done. Fixtures in $FIXTURES_DIR/"
echo "Remember to pin commits to specific SHAs for reproducibility."
