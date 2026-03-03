#!/usr/bin/env bash
# 07-link-integrity.sh — VAL-043 through VAL-045
# Checks: relative markdown links resolve, BEH/ADR link format

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

# Directories to skip
SKIP_DIRS="visual references scripts"

# ─── Collect all .md files ───────────────────────────────────────────────────

files_to_check=$(make_temp)
for f in "$SPEC_DIR"/*.md "$SPEC_DIR"/*/*.md; do
  [[ -f "$f" ]] || continue
  # Skip dirs
  rel="${f#"$SPEC_DIR"/}"
  if echo "$rel" | grep -q '/'; then
    dir_part=$(echo "$rel" | cut -d'/' -f1)
    skip=false
    for sd in $SKIP_DIRS; do
      [[ "$dir_part" == "$sd" ]] && skip=true && break
    done
    [[ "$skip" == "true" ]] && continue
  fi
  echo "$f" >> "$files_to_check"
done

# ─── VAL-043: Relative markdown links resolve ───────────────────────────────

file_count=0
while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  rel=$(rel_path "$file" "$SPEC_DIR")
  file_dir=$(dirname "$file")

  # Extract relative links: ](path) but not ](#anchor) and not ](http
  # Use grep -E (no -P for macOS compat)
  grep -Eo '\]\([^)]+\)' "$file" 2>/dev/null | while IFS= read -r match; do
    # Extract the URL part
    link=$(echo "$match" | sed 's/^\](//' | sed 's/)$//')

    # Skip fragment-only links
    [[ "$link" == \#* ]] && continue
    # Skip external URLs
    [[ "$link" == http://* || "$link" == https://* ]] && continue
    # Skip mailto links
    [[ "$link" == mailto:* ]] && continue

    # Strip fragment from link (path#section → path)
    link_path=$(echo "$link" | sed 's/#.*//')
    [[ -z "$link_path" ]] && continue

    # Resolve relative to file's directory
    if [[ "$link_path" == /* ]]; then
      target="$link_path"
    else
      target="$file_dir/$link_path"
    fi

    # Normalize path (resolve . and ..)
    if [[ -e "$target" ]]; then
      continue
    fi

    # Try resolving with cd/pwd
    target_dir=$(dirname "$target")
    target_base=$(basename "$target")
    if [[ -d "$target_dir" ]]; then
      resolved_dir=$(cd "$target_dir" 2>/dev/null && pwd)
      if [[ -e "$resolved_dir/$target_base" ]]; then
        continue
      fi
    fi

    emit "VAL-043" "error" "$rel" "Broken link: [$link] does not resolve to an existing file"
  done

  ((file_count++)) || true
done < "$files_to_check"

# ─── VAL-044: BEH link filename format ──────────────────────────────────────

if has_dir "$SPEC_DIR/behaviors"; then
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    rel=$(rel_path "$file" "$SPEC_DIR")

    # Find links that reference behavior files
    grep -Eo '\]\([^)]*BEH-[^)]+\)' "$file" 2>/dev/null | while IFS= read -r match; do
      link=$(echo "$match" | sed 's/^\](//' | sed 's/)$//')
      # Strip fragment
      link_path=$(echo "$link" | sed 's/#.*//')
      [[ -z "$link_path" ]] && continue
      link_base=$(basename "$link_path")

      # Check format: BEH-{INFIX}-NNN-slug.md
      if ! echo "$link_base" | grep -qE "^BEH-${INFIX}-[0-9]+-[a-z0-9-]+\.md$"; then
        emit "VAL-044" "warning" "$rel" "BEH link '$link_base' does not match expected format BEH-${INFIX}-NNN-slug.md"
      fi
    done
  done < "$files_to_check"
fi

# ─── VAL-045: ADR link filename format ──────────────────────────────────────

if has_dir "$SPEC_DIR/decisions"; then
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    rel=$(rel_path "$file" "$SPEC_DIR")

    # Find links that reference ADR files
    grep -Eo '\]\([^)]*ADR-[^)]+\)' "$file" 2>/dev/null | while IFS= read -r match; do
      link=$(echo "$match" | sed 's/^\](//' | sed 's/)$//')
      # Strip fragment
      link_path=$(echo "$link" | sed 's/#.*//')
      [[ -z "$link_path" ]] && continue
      link_base=$(basename "$link_path")

      # Check format: ADR-NNN-slug.md
      if ! echo "$link_base" | grep -qE '^ADR-[0-9]+-[a-z0-9-]+\.md$'; then
        emit "VAL-045" "warning" "$rel" "ADR link '$link_base' does not match expected format ADR-NNN-slug.md"
      fi
    done
  done < "$files_to_check"
fi

finalize "07-link-integrity (VAL-043..045)"
