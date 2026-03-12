#!/usr/bin/env python3
"""
Regenerate .expected files from .expect.md files.

The .expect.md files come from the upstream React compiler repo and contain
the authoritative expected outputs. The previous run-upstream.mjs approach
was incorrect because transformSync returns reformatted source even for
bail-out cases, losing the actual compiler output.

This script:
1. Extracts the ## Code section from each .expect.md file → .expected
2. Marks error-only .expect.md files (## Error but no ## Code) with UPSTREAM ERROR marker
3. Walks subdirectories recursively
"""

import os
import re
import sys

fixtures_dir = os.path.join(os.path.dirname(__file__), "upstream-fixtures")

if not os.path.isdir(fixtures_dir):
    print(f"Fixtures directory not found: {fixtures_dir}", file=sys.stderr)
    sys.exit(1)

# Pattern to extract code from ## Code section
CODE_PATTERN = re.compile(
    r"## Code\s*\n+```(?:javascript|typescript|jsx|tsx)?\s*\n(.*?)```",
    re.DOTALL,
)

updated = 0
skipped = 0
errors = 0
no_code = 0
error_marked = 0

def process_dir(directory):
    global updated, skipped, errors, no_code, error_marked

    for entry in sorted(os.listdir(directory)):
        full_path = os.path.join(directory, entry)

        if os.path.isdir(full_path):
            process_dir(full_path)
            continue

        if not entry.endswith(".expect.md"):
            continue

        md_path = full_path
        # Derive expected path: foo.expect.md -> foo.expected
        base = entry.replace(".expect.md", "")
        expected_path = os.path.join(directory, base + ".expected")

        try:
            with open(md_path, "r") as f:
                content = f.read()
        except Exception as e:
            print(f"ERROR reading {md_path}: {e}", file=sys.stderr)
            errors += 1
            continue

        match = CODE_PATTERN.search(content)
        if not match:
            # Error-only fixture: mark with UPSTREAM ERROR
            no_code += 1
            if "## Error" in content:
                idx = content.find("## Error")
                error_section = content[idx:idx+200].replace("\n", " ")
                marker = f"// UPSTREAM ERROR: {error_section[:150]}\n"
                # Only write if different
                if os.path.exists(expected_path):
                    existing = open(expected_path).read()
                    if existing == marker:
                        continue
                with open(expected_path, "w") as ef:
                    ef.write(marker)
                error_marked += 1
            continue

        code = match.group(1)
        code = code.rstrip() + "\n"

        # Check if .expected file already has correct content
        if os.path.exists(expected_path):
            try:
                with open(expected_path, "r") as f:
                    existing = f.read()
                if existing == code:
                    skipped += 1
                    continue
            except:
                pass

        with open(expected_path, "w") as f:
            f.write(code)
        updated += 1

process_dir(fixtures_dir)

print(f"Updated: {updated}")
print(f"Unchanged: {skipped}")
print(f"No ## Code section: {no_code} (marked as error: {error_marked})")
print(f"Errors: {errors}")
print(f"Total .expect.md files processed: {updated + skipped + no_code + errors}")
