#!/usr/bin/env bash
# verify-libraries.sh — CHK-LIB-001 through CHK-LIB-005
# Checks: library cross-references to features, other libraries, npm packages, and family consistency
# Sources shared utilities from spec-review/scripts/lib.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REVIEW_LIB="$SCRIPT_DIR/../../spec-review/scripts/lib.sh"

if [[ ! -f "$REVIEW_LIB" ]]; then
  echo "ERROR: Cannot find spec-review/scripts/lib.sh at $REVIEW_LIB" >&2
  exit 1
fi

source "$REVIEW_LIB"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

# ─── Graceful skip if libraries/ absent ──────────────────────────────────────

if [[ ! -d "$SPEC_DIR/libraries" ]]; then
  echo "info: No libraries/ directory in $SPEC_DIR — skipping all checks" >&2
  exit 0
fi

# ─── Helper: collect LIB IDs from sub-folder structure ───────────────────────

collect_lib_ids() {
  local lib_dir="$1"
  for f in "$lib_dir"/*/*.md "$lib_dir"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    lid=$(get_fm_field "$f" "id")
    [[ -n "$lid" ]] && echo "$lid"
  done | sort -u
}

# ─── Collect existing FEAT IDs from features/ ───────────────────────────────

feat_ids=$(make_temp)
if has_dir "$SPEC_DIR/features"; then
  for f in "$SPEC_DIR/features"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    fid=$(get_fm_field "$f" "id")
    [[ -n "$fid" ]] && echo "$fid"
  done | sort -u > "$feat_ids"
fi

# ─── Collect existing LIB IDs ───────────────────────────────────────────────

lib_ids=$(make_temp)
collect_lib_ids "$SPEC_DIR/libraries" > "$lib_ids"

# ─── Detect monorepo root ───────────────────────────────────────────────────

REPO_ROOT=""
# Walk up from spec dir to find package.json at monorepo root
dir="$SPEC_DIR"
while [[ "$dir" != "/" ]]; do
  if [[ -f "$dir/pnpm-workspace.yaml" ]] || [[ -f "$dir/turbo.json" ]]; then
    REPO_ROOT="$dir"
    break
  fi
  dir=$(dirname "$dir")
done

# ─── Iterate over library files ─────────────────────────────────────────────

for f in "$SPEC_DIR/libraries"/*/*.md "$SPEC_DIR/libraries"/*.md; do
  [[ -f "$f" ]] || continue
  basename_f=$(basename "$f")
  [[ "$basename_f" == "index.md" ]] && continue

  # Determine relative path within libraries/
  rel_from_libs="${f#"$SPEC_DIR/libraries/"}"
  rel="libraries/$rel_from_libs"

  # Determine family from path (directory name if in sub-folder)
  family_from_path=""
  if echo "$rel_from_libs" | grep -q '/'; then
    family_from_path=$(echo "$rel_from_libs" | cut -d'/' -f1)
  fi

  # ── CHK-LIB-001: features[] → existing FEAT ──
  if has_dir "$SPEC_DIR/features"; then
    while IFS= read -r feat_ref; do
      [[ -z "$feat_ref" ]] && continue
      if ! grep -qx "$feat_ref" "$feat_ids" 2>/dev/null; then
        emit "CHK-LIB-001" "error" "$rel" "features ref '$feat_ref' does not match any feature in features/"
      fi
    done < <(get_fm_list "$f" "features")
  else
    skip_rule "CHK-LIB-001" "No features/ directory for FEAT resolution"
  fi

  # ── CHK-LIB-002: depends_on[] → existing LIB ──
  while IFS= read -r lib_ref; do
    [[ -z "$lib_ref" ]] && continue
    if ! grep -qx "$lib_ref" "$lib_ids" 2>/dev/null; then
      emit "CHK-LIB-002" "error" "$rel" "depends_on ref '$lib_ref' does not match any library in libraries/"
    fi
  done < <(get_fm_list "$f" "depends_on")

  # ── CHK-LIB-003: npm_name matches actual package.json ──
  npm_name=$(get_fm_field "$f" "npm_name")
  lib_path=$(get_fm_field "$f" "path")
  if [[ -n "$npm_name" && -n "$lib_path" && -n "$REPO_ROOT" ]]; then
    pkg_json="$REPO_ROOT/$lib_path/package.json"
    if [[ -f "$pkg_json" ]]; then
      actual_name=$(grep -o '"name"[[:space:]]*:[[:space:]]*"[^"]*"' "$pkg_json" | head -1 | sed 's/.*"name"[[:space:]]*:[[:space:]]*"//' | sed 's/"$//' || true)
      if [[ -n "$actual_name" && "$actual_name" != "$npm_name" ]]; then
        emit "CHK-LIB-003" "warning" "$rel" \
          "npm_name '$npm_name' does not match package.json name '$actual_name' at $lib_path/package.json"
      fi
    else
      emit "CHK-LIB-003" "warning" "$rel" \
        "package.json not found at $lib_path/package.json (path: $pkg_json)"
    fi
  elif [[ -z "$REPO_ROOT" ]]; then
    skip_rule "CHK-LIB-003" "Cannot detect monorepo root for package.json validation"
  fi

  # ── CHK-LIB-004: family sub-folder matches family frontmatter ──
  family_fm=$(get_fm_field "$f" "family")
  if [[ -n "$family_from_path" && -n "$family_fm" ]]; then
    if [[ "$family_from_path" != "$family_fm" ]]; then
      emit "CHK-LIB-004" "warning" "$rel" \
        "family frontmatter '$family_fm' does not match sub-folder name '$family_from_path'"
    fi
  fi

  # ── CHK-LIB-005: depends_on matches actual @hex-di/ dependencies ──
  if [[ -n "$lib_path" && -n "$REPO_ROOT" ]]; then
    pkg_json="$REPO_ROOT/$lib_path/package.json"
    if [[ -f "$pkg_json" ]]; then
      # Extract @hex-di/ dependencies from package.json
      actual_deps=$(make_temp)
      grep -o '"@hex-di/[^"]*"' "$pkg_json" | sed 's/"//g' | sort -u > "$actual_deps" 2>/dev/null || true

      # Extract npm_name values from depends_on libraries
      declared_deps=$(make_temp)
      while IFS= read -r lib_ref; do
        [[ -z "$lib_ref" ]] && continue
        # Find the referenced library's npm_name
        for dep_file in "$SPEC_DIR/libraries"/*/*.md "$SPEC_DIR/libraries"/*.md; do
          [[ -f "$dep_file" ]] || continue
          dep_id=$(get_fm_field "$dep_file" "id")
          if [[ "$dep_id" == "$lib_ref" ]]; then
            dep_npm=$(get_fm_field "$dep_file" "npm_name")
            [[ -n "$dep_npm" ]] && echo "$dep_npm"
            break
          fi
        done
      done < <(get_fm_list "$f" "depends_on") | sort -u > "$declared_deps"

      # Report @hex-di/ deps in package.json but not in depends_on
      if [[ -s "$actual_deps" ]]; then
        while IFS= read -r actual_dep; do
          [[ -z "$actual_dep" ]] && continue
          if ! grep -qx "$actual_dep" "$declared_deps" 2>/dev/null; then
            emit "CHK-LIB-005" "info" "$rel" \
              "package.json dependency '$actual_dep' not represented in depends_on[] (may be transitive)"
          fi
        done < "$actual_deps"
      fi
    fi
  fi

done

finalize "verify-libraries (CHK-LIB-001..005)"
