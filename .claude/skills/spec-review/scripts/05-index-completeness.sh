#!/usr/bin/env bash
# 05-index-completeness.sh — VAL-031 through VAL-037
# Checks: index.yaml ↔ filesystem synchronization

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")

# ─── Generic index check ────────────────────────────────────────────────────

# Usage: check_index <dir> <dir_label> <rule_missing> <severity_missing> <rule_stale>
check_index() {
  local dir="$1"
  local dir_label="$2"
  local rule_missing="$3"
  local severity_missing="$4"

  local yaml="$dir/index.yaml"
  if [[ ! -f "$yaml" ]]; then
    skip_rule "$rule_missing" "No index.yaml in $dir_label/"
    return
  fi

  # Get files listed in index.yaml
  local index_files
  index_files=$(make_temp)
  extract_index_files "$yaml" | sort -u > "$index_files"

  # Get actual .md files in directory
  local actual_files
  actual_files=$(make_temp)
  list_md_files "$dir" | sort > "$actual_files"

  # Files in directory but not in index (VAL-031..036)
  while IFS= read -r actual; do
    [[ -z "$actual" ]] && continue
    if ! grep -qx "$actual" "$index_files" 2>/dev/null; then
      emit "$rule_missing" "$severity_missing" "$dir_label/$actual" \
        "File exists in $dir_label/ but not listed in $dir_label/index.yaml"
    fi
  done < "$actual_files"

  # Files in index but not in directory (VAL-037)
  while IFS= read -r indexed; do
    [[ -z "$indexed" ]] && continue
    if [[ ! -f "$dir/$indexed" ]]; then
      emit "VAL-037" "error" "$dir_label/index.yaml" \
        "Stale index entry: '$indexed' listed but file does not exist in $dir_label/"
    fi
  done < "$index_files"
}

# ─── VAL-031: behaviors/ files listed in index.yaml ──────────────────────────

if has_dir "$SPEC_DIR/behaviors"; then
  check_index "$SPEC_DIR/behaviors" "behaviors" "VAL-031" "error"
else
  skip_rule "VAL-031" "No behaviors/ directory"
fi

# ─── VAL-032: decisions/ files listed in index.yaml ──────────────────────────

if has_dir "$SPEC_DIR/decisions"; then
  check_index "$SPEC_DIR/decisions" "decisions" "VAL-032" "error"
else
  skip_rule "VAL-032" "No decisions/ directory"
fi

# ─── VAL-033: types/ files listed in index.yaml ─────────────────────────────

if has_dir "$SPEC_DIR/types"; then
  check_index "$SPEC_DIR/types" "types" "VAL-033" "error"
else
  skip_rule "VAL-033" "No types/ directory"
fi

# ─── VAL-034: features/ files listed in index.yaml ──────────────────────────

if has_dir "$SPEC_DIR/features"; then
  check_index "$SPEC_DIR/features" "features" "VAL-034" "warning"
else
  skip_rule "VAL-034" "No features/ directory"
fi

# ─── VAL-035: capabilities/ files listed in index.yaml ──────────────────────
# Capabilities may use optional group sub-folders, so handle both flat and nested

if has_dir "$SPEC_DIR/capabilities"; then
  cap_yaml="$SPEC_DIR/capabilities/index.yaml"
  if [[ -f "$cap_yaml" ]]; then
    # Get files listed in index.yaml
    cap_index_files=$(make_temp)
    extract_index_files "$cap_yaml" | sort -u > "$cap_index_files"

    # Get actual .md files in capabilities/ (flat and sub-folders)
    cap_actual_files=$(make_temp)
    for f in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
      [[ -f "$f" ]] || continue
      basename_f=$(basename "$f")
      [[ "$basename_f" == "index.md" ]] && continue
      # Relative path from capabilities/ (e.g., file.md or group/file.md)
      rel_from_caps="${f#"$SPEC_DIR/capabilities/"}"
      echo "$rel_from_caps"
    done | sort > "$cap_actual_files"

    # Files on disk but not in index
    while IFS= read -r actual; do
      [[ -z "$actual" ]] && continue
      if ! grep -qx "$actual" "$cap_index_files" 2>/dev/null; then
        emit "VAL-035" "warning" "capabilities/$actual" \
          "File exists in capabilities/ but not listed in capabilities/index.yaml"
      fi
    done < "$cap_actual_files"

    # Files in index but not on disk
    while IFS= read -r indexed; do
      [[ -z "$indexed" ]] && continue
      if [[ ! -f "$SPEC_DIR/capabilities/$indexed" ]]; then
        emit "VAL-037" "error" "capabilities/index.yaml" \
          "Stale index entry: '$indexed' listed but file does not exist in capabilities/"
      fi
    done < "$cap_index_files"
  else
    skip_rule "VAL-035" "No index.yaml in capabilities/"
  fi
else
  skip_rule "VAL-035" "No capabilities/ directory"
fi

# ─── VAL-036: invariants/ files listed in index.yaml ────────────────────────

if has_dir "$SPEC_DIR/invariants"; then
  check_index "$SPEC_DIR/invariants" "invariants" "VAL-036" "warning"
else
  skip_rule "VAL-036" "No invariants/ directory"
fi

# Note: VAL-037 (stale entries) is checked within each check_index call above

# ─── VAL-062: deliverables/ and libraries/ files listed in index.yaml ────────

# Deliverables use flat structure (standard check)
if has_dir "$SPEC_DIR/deliverables"; then
  check_index "$SPEC_DIR/deliverables" "deliverables" "VAL-062" "warning"
else
  skip_rule "VAL-062" "No deliverables/ directory"
fi

# Libraries use family sub-folders — check with sub-folder handling
if has_dir "$SPEC_DIR/libraries"; then
  lib_yaml="$SPEC_DIR/libraries/index.yaml"
  if [[ -f "$lib_yaml" ]]; then
    # Get files listed in index.yaml (paths include sub-folder prefix)
    lib_index_files=$(make_temp)
    extract_index_files "$lib_yaml" | sort -u > "$lib_index_files"

    # Get actual .md files in all sub-folders
    lib_actual_files=$(make_temp)
    for f in "$SPEC_DIR/libraries"/*/*.md "$SPEC_DIR/libraries"/*.md; do
      [[ -f "$f" ]] || continue
      basename_f=$(basename "$f")
      [[ "$basename_f" == "index.md" ]] && continue
      # Relative path from libraries/ (e.g., core/LIB-SF-001-di-kernel.md)
      rel_from_libs="${f#"$SPEC_DIR/libraries/"}"
      echo "$rel_from_libs"
    done | sort > "$lib_actual_files"

    # Files on disk but not in index
    while IFS= read -r actual; do
      [[ -z "$actual" ]] && continue
      if ! grep -qx "$actual" "$lib_index_files" 2>/dev/null; then
        emit "VAL-062" "warning" "libraries/$actual" \
          "File exists in libraries/ but not listed in libraries/index.yaml"
      fi
    done < "$lib_actual_files"

    # Files in index but not on disk
    while IFS= read -r indexed; do
      [[ -z "$indexed" ]] && continue
      if [[ ! -f "$SPEC_DIR/libraries/$indexed" ]]; then
        emit "VAL-037" "error" "libraries/index.yaml" \
          "Stale index entry: '$indexed' listed but file does not exist in libraries/"
      fi
    done < "$lib_index_files"
  else
    skip_rule "VAL-062" "No index.yaml in libraries/"
  fi
else
  skip_rule "VAL-062" "No libraries/ directory"
fi

finalize "05-index-completeness (VAL-031..037, VAL-062)"
