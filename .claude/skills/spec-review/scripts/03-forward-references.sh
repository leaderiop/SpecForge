#!/usr/bin/env bash
# 03-forward-references.sh — VAL-018 through VAL-024
# Checks: all cross-references in frontmatter point to existing targets

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

# ─── Build entity indexes ───────────────────────────────────────────────────

# Collect all INV IDs (from invariants/ frontmatter)
inv_ids=$(make_temp)
if has_dir "$SPEC_DIR/invariants"; then
  for f in "$SPEC_DIR/invariants"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    iid=$(get_fm_field "$f" "id")
    [[ -n "$iid" ]] && echo "$iid" >> "$inv_ids"
  done
fi

# Collect all ADR IDs (from decisions/ frontmatter)
adr_ids=$(make_temp)
if has_dir "$SPEC_DIR/decisions"; then
  for f in "$SPEC_DIR/decisions"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    aid=$(get_fm_field "$f" "id")
    [[ -n "$aid" ]] && echo "$aid" >> "$adr_ids"
  done
fi

# Collect all BEH IDs (from ## headers in behaviors/ and plugins/)
beh_ids=$(make_temp)
beh_ranges=$(make_temp)
if has_dir "$SPEC_DIR/behaviors" || has_dir "$SPEC_DIR/plugins"; then
  collect_beh_ids "$SPEC_DIR" "$INFIX" | cut -f1 | sort -u > "$beh_ids"

  # Also collect id_range info for range-based resolution
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    id_range=$(get_fm_field "$f" "id_range")
    [[ -z "$id_range" ]] && continue
    range_lo=$(echo "$id_range" | sed -E 's/^"?([0-9]+)[-–]+.*/\1/' | sed 's/^0*//')
    range_hi=$(echo "$id_range" | sed -E 's/.*[-–]+([0-9]+)"?$/\1/' | sed 's/^0*//')
    [[ -n "$range_lo" && -n "$range_hi" ]] && printf '%s\t%s\n' "$range_lo" "$range_hi" >> "$beh_ranges"
  done
fi

# Collect all FEAT IDs (from features/ frontmatter)
feat_ids=$(make_temp)
if has_dir "$SPEC_DIR/features"; then
  for f in "$SPEC_DIR/features"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    fid=$(get_fm_field "$f" "id")
    [[ -n "$fid" ]] && echo "$fid" >> "$feat_ids"
  done
fi

# Collect all UX IDs (from capabilities/ frontmatter)
ux_ids=$(make_temp)
if has_dir "$SPEC_DIR/capabilities"; then
  for f in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    uid=$(get_fm_field "$f" "id")
    [[ -n "$uid" ]] && echo "$uid" >> "$ux_ids"
  done
fi

# Collect all LIB IDs (from libraries/ sub-folders)
lib_ids=$(make_temp)
if has_dir "$SPEC_DIR/libraries"; then
  collect_lib_file_ids "$SPEC_DIR/libraries" | cut -f1 | sort -u > "$lib_ids"
fi

# Collect type files (from types/)
type_files=$(make_temp)
if has_dir "$SPEC_DIR/types"; then
  for f in "$SPEC_DIR/types"/*.md; do
    [[ -f "$f" ]] || continue
    local_bn=$(basename "$f" .md)
    [[ "$local_bn" == "index" ]] && continue
    echo "$local_bn" >> "$type_files"
  done
fi

# ─── BEH ID resolution helper ───────────────────────────────────────────────

# Returns 0 if BEH ID exists (by direct match or range coverage)
beh_id_exists() {
  local id="$1"
  # Direct match in collected headers
  if grep -qx "$id" "$beh_ids" 2>/dev/null; then
    return 0
  fi
  # Range-based resolution
  local num
  num=$(echo "$id" | grep -Eo '[0-9]+$' | sed 's/^0*//')
  [[ -z "$num" ]] && return 1
  if [[ -s "$beh_ranges" ]]; then
    while IFS=$'\t' read -r lo hi; do
      if [[ "$num" -ge "$lo" && "$num" -le "$hi" ]]; then
        return 0
      fi
    done < "$beh_ranges"
  fi
  return 1
}

# ─── VAL-018: invariants[] → existing INV ────────────────────────────────────

if has_dir "$SPEC_DIR/behaviors"; then
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="behaviors/$(basename "$f")"
    while IFS= read -r inv_ref; do
      [[ -z "$inv_ref" ]] && continue
      if ! grep -qx "$inv_ref" "$inv_ids" 2>/dev/null; then
        emit "VAL-018" "error" "$rel" "invariants ref '$inv_ref' does not match any invariant in invariants/"
      fi
    done < <(get_fm_list "$f" "invariants")
  done
else
  skip_rule "VAL-018" "No behaviors/ directory"
fi

# ─── VAL-019: adrs[] → existing ADR ─────────────────────────────────────────

check_adr_refs() {
  local dir="$1" dir_label="$2"
  [[ -d "$dir" ]] || return 0
  for f in "$dir"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    local rel="$dir_label/$(basename "$f")"
    while IFS= read -r adr_ref; do
      [[ -z "$adr_ref" ]] && continue
      if ! grep -qx "$adr_ref" "$adr_ids" 2>/dev/null; then
        emit "VAL-019" "error" "$rel" "adrs ref '$adr_ref' does not match any decision in decisions/"
      fi
    done < <(get_fm_list "$f" "adrs")
  done
}

if has_dir "$SPEC_DIR/decisions"; then
  check_adr_refs "$SPEC_DIR/behaviors" "behaviors"
  check_adr_refs "$SPEC_DIR/features" "features"
  check_adr_refs "$SPEC_DIR/types" "types"
else
  skip_rule "VAL-019" "No decisions/ directory for ADR resolution"
fi

# ─── VAL-020: behaviors[] → existing BEH ────────────────────────────────────

check_beh_refs() {
  local dir="$1" dir_label="$2"
  [[ -d "$dir" ]] || return 0
  for f in "$dir"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    local rel="$dir_label/$(basename "$f")"
    while IFS= read -r beh_ref; do
      [[ -z "$beh_ref" ]] && continue
      if ! beh_id_exists "$beh_ref"; then
        emit "VAL-020" "error" "$rel" "behaviors ref '$beh_ref' does not resolve to any behavior definition"
      fi
    done < <(get_fm_list "$f" "behaviors")
  done
}

if [[ -s "$beh_ids" ]] || [[ -s "$beh_ranges" ]]; then
  check_beh_refs "$SPEC_DIR/features" "features"
  check_beh_refs "$SPEC_DIR/capabilities" "capabilities"
else
  skip_rule "VAL-020" "No behavior IDs collected for resolution"
fi

# ─── VAL-021: features[] → existing FEAT ────────────────────────────────────

if has_dir "$SPEC_DIR/features" && has_dir "$SPEC_DIR/capabilities"; then
  for f in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="${f#"$SPEC_DIR"/}"
    while IFS= read -r feat_ref; do
      [[ -z "$feat_ref" ]] && continue
      if ! grep -qx "$feat_ref" "$feat_ids" 2>/dev/null; then
        emit "VAL-021" "error" "$rel" "features ref '$feat_ref' does not match any feature in features/"
      fi
    done < <(get_fm_list "$f" "features")
  done
else
  skip_rule "VAL-021" "No features/ or capabilities/ directory"
fi

# ─── VAL-022: types[] → existing type file ───────────────────────────────────

if has_dir "$SPEC_DIR/types" && has_dir "$SPEC_DIR/behaviors"; then
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="behaviors/$(basename "$f")"
    while IFS= read -r type_ref; do
      [[ -z "$type_ref" ]] && continue
      if ! grep -qx "$type_ref" "$type_files" 2>/dev/null; then
        emit "VAL-022" "warning" "$rel" "types ref '$type_ref' does not match any file in types/"
      fi
    done < <(get_fm_list "$f" "types")
  done
else
  skip_rule "VAL-022" "No types/ or behaviors/ directory"
fi

# ─── VAL-023: supersedes[] → existing ADR ────────────────────────────────────

if has_dir "$SPEC_DIR/decisions"; then
  for f in "$SPEC_DIR/decisions"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="decisions/$(basename "$f")"
    while IFS= read -r sup_ref; do
      [[ -z "$sup_ref" ]] && continue
      if ! grep -qx "$sup_ref" "$adr_ids" 2>/dev/null; then
        emit "VAL-023" "error" "$rel" "supersedes ref '$sup_ref' does not match any decision in decisions/"
      fi
    done < <(get_fm_list "$f" "supersedes")
  done
else
  skip_rule "VAL-023" "No decisions/ directory"
fi

# ─── VAL-024: roadmap_phases[] → existing roadmap ────────────────────────────

if has_dir "$SPEC_DIR/features"; then
  # Collect roadmap phase IDs from roadmap/ files
  roadmap_ids=$(make_temp)
  if has_dir "$SPEC_DIR/roadmap"; then
    for f in "$SPEC_DIR/roadmap"/*.md; do
      [[ -f "$f" ]] || continue
      # Extract RM-NN or PH-N patterns from filenames and content
      basename_f=$(basename "$f")
      echo "$basename_f" | grep -Eo 'RM-[0-9]+' >> "$roadmap_ids" 2>/dev/null || true
      grep -Eo 'RM-[0-9]+' "$f" >> "$roadmap_ids" 2>/dev/null || true
      grep -Eo '## Phase [0-9]+' "$f" | sed -E 's/## Phase ([0-9]+)/RM-\1/' >> "$roadmap_ids" 2>/dev/null || true
    done
    sort -u -o "$roadmap_ids" "$roadmap_ids"
  fi

  if [[ -s "$roadmap_ids" ]]; then
    for f in "$SPEC_DIR/features"/*.md; do
      [[ -f "$f" ]] || continue
      [[ "$(basename "$f")" == "index.md" ]] && continue
      rel="features/$(basename "$f")"
      while IFS= read -r rm_ref; do
        [[ -z "$rm_ref" ]] && continue
        # Normalize RM-01 to RM-1 for comparison
        rm_norm=$(echo "$rm_ref" | sed -E 's/RM-0*([0-9]+)/RM-\1/')
        found=false
        while IFS= read -r known; do
          known_norm=$(echo "$known" | sed -E 's/RM-0*([0-9]+)/RM-\1/')
          if [[ "$rm_norm" == "$known_norm" ]]; then
            found=true
            break
          fi
        done < "$roadmap_ids"
        if [[ "$found" == "false" ]]; then
          emit "VAL-024" "warning" "$rel" "roadmap_phases ref '$rm_ref' not found in roadmap/"
        fi
      done < <(get_fm_list "$f" "roadmap_phases")
    done
  else
    skip_rule "VAL-024" "No roadmap/ directory or no roadmap phase IDs found"
  fi
else
  skip_rule "VAL-024" "No features/ directory"
fi

# ─── VAL-058: Deliverable capabilities[] → existing UX ──────────────────────

if has_dir "$SPEC_DIR/deliverables" && [[ -s "$ux_ids" ]]; then
  for f in "$SPEC_DIR/deliverables"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="deliverables/$(basename "$f")"
    while IFS= read -r cap_ref; do
      [[ -z "$cap_ref" ]] && continue
      if ! grep -qx "$cap_ref" "$ux_ids" 2>/dev/null; then
        emit "VAL-058" "error" "$rel" "capabilities ref '$cap_ref' does not match any capability in capabilities/"
      fi
    done < <(get_fm_list "$f" "capabilities")
  done
elif has_dir "$SPEC_DIR/deliverables"; then
  skip_rule "VAL-058" "No capabilities/ directory for UX resolution"
else
  skip_rule "VAL-058" "No deliverables/ directory"
fi

# ─── VAL-059: Deliverable depends_on[] → existing LIB ───────────────────────

if has_dir "$SPEC_DIR/deliverables" && [[ -s "$lib_ids" ]]; then
  for f in "$SPEC_DIR/deliverables"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="deliverables/$(basename "$f")"
    while IFS= read -r lib_ref; do
      [[ -z "$lib_ref" ]] && continue
      if ! grep -qx "$lib_ref" "$lib_ids" 2>/dev/null; then
        emit "VAL-059" "error" "$rel" "depends_on ref '$lib_ref' does not match any library in libraries/"
      fi
    done < <(get_fm_list "$f" "depends_on")
  done
elif has_dir "$SPEC_DIR/deliverables"; then
  skip_rule "VAL-059" "No libraries/ directory for LIB resolution"
else
  skip_rule "VAL-059" "No deliverables/ directory"
fi

# ─── VAL-060: Library features[] → existing FEAT ────────────────────────────

if has_dir "$SPEC_DIR/libraries" && [[ -s "$feat_ids" ]]; then
  for f in "$SPEC_DIR/libraries"/*/*.md "$SPEC_DIR/libraries"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="libraries/${f#"$SPEC_DIR/libraries/"}"
    while IFS= read -r feat_ref; do
      [[ -z "$feat_ref" ]] && continue
      if ! grep -qx "$feat_ref" "$feat_ids" 2>/dev/null; then
        emit "VAL-060" "error" "$rel" "features ref '$feat_ref' does not match any feature in features/"
      fi
    done < <(get_fm_list "$f" "features")
  done
elif has_dir "$SPEC_DIR/libraries"; then
  skip_rule "VAL-060" "No features/ directory for FEAT resolution"
else
  skip_rule "VAL-060" "No libraries/ directory"
fi

# ─── VAL-061: Library depends_on[] → existing LIB ───────────────────────────

if has_dir "$SPEC_DIR/libraries" && [[ -s "$lib_ids" ]]; then
  for f in "$SPEC_DIR/libraries"/*/*.md "$SPEC_DIR/libraries"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="libraries/${f#"$SPEC_DIR/libraries/"}"
    while IFS= read -r lib_dep_ref; do
      [[ -z "$lib_dep_ref" ]] && continue
      if ! grep -qx "$lib_dep_ref" "$lib_ids" 2>/dev/null; then
        emit "VAL-061" "error" "$rel" "depends_on ref '$lib_dep_ref' does not match any library in libraries/"
      fi
    done < <(get_fm_list "$f" "depends_on")
  done
else
  skip_rule "VAL-061" "No libraries/ directory or no LIB IDs"
fi

finalize "03-forward-references (VAL-018..024, VAL-058..061)"
