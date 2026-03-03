#!/usr/bin/env bash
# 04-reverse-coverage.sh — VAL-025 through VAL-030
# Checks: orphan detection — every entity referenced by at least one upstream entity

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

# ─── Collect all entity IDs ─────────────────────────────────────────────────

# All BEH IDs from headers
all_beh=$(make_temp)
if has_dir "$SPEC_DIR/behaviors" || has_dir "$SPEC_DIR/plugins"; then
  collect_beh_ids "$SPEC_DIR" "$INFIX" | cut -f1 | sort -u > "$all_beh"
fi

# All FEAT IDs
all_feat=$(make_temp)
if has_dir "$SPEC_DIR/features"; then
  for f in "$SPEC_DIR/features"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    fid=$(get_fm_field "$f" "id")
    [[ -n "$fid" ]] && echo "$fid"
  done | sort -u > "$all_feat"
fi

# All INV IDs
all_inv=$(make_temp)
if has_dir "$SPEC_DIR/invariants"; then
  for f in "$SPEC_DIR/invariants"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    iid=$(get_fm_field "$f" "id")
    [[ -n "$iid" ]] && echo "$iid"
  done | sort -u > "$all_inv"
fi

# All ADR IDs
all_adr=$(make_temp)
if has_dir "$SPEC_DIR/decisions"; then
  for f in "$SPEC_DIR/decisions"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    aid=$(get_fm_field "$f" "id")
    [[ -n "$aid" ]] && echo "$aid"
  done | sort -u > "$all_adr"
fi

# All type domains
all_types=$(make_temp)
if has_dir "$SPEC_DIR/types"; then
  for f in "$SPEC_DIR/types"/*.md; do
    [[ -f "$f" ]] || continue
    local_bn=$(basename "$f" .md)
    [[ "$local_bn" == "index" ]] && continue
    echo "$local_bn"
  done | sort -u > "$all_types"
fi

# ─── Collect all upstream references ────────────────────────────────────────

# BEH IDs referenced by features (behaviors[] field)
referenced_beh=$(make_temp)
if has_dir "$SPEC_DIR/features"; then
  for f in "$SPEC_DIR/features"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "behaviors"
  done | sort -u > "$referenced_beh"
fi

# FEAT IDs referenced by capabilities (features[] field)
referenced_feat=$(make_temp)
if has_dir "$SPEC_DIR/capabilities"; then
  for f in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "features"
  done | sort -u > "$referenced_feat"
fi

# INV IDs referenced by behaviors (invariants[] field)
referenced_inv=$(make_temp)
if has_dir "$SPEC_DIR/behaviors"; then
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "invariants"
  done | sort -u > "$referenced_inv"
fi

# ADR IDs referenced by behaviors (adrs[] field)
referenced_adr=$(make_temp)
if has_dir "$SPEC_DIR/behaviors"; then
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "adrs"
  done | sort -u > "$referenced_adr"
fi

# Type domains referenced by behaviors (types[] field)
referenced_types=$(make_temp)
if has_dir "$SPEC_DIR/behaviors"; then
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "types"
  done | sort -u > "$referenced_types"
fi

# ─── VAL-025: Every BEH referenced by a feature ─────────────────────────────

if has_dir "$SPEC_DIR/features" && [[ -s "$all_beh" ]]; then
  while IFS= read -r beh_id; do
    if ! grep -qx "$beh_id" "$referenced_beh" 2>/dev/null; then
      emit "VAL-025" "warning" "behaviors/" "Orphan behavior: $beh_id not referenced by any feature's behaviors[] list"
    fi
  done < "$all_beh"
else
  skip_rule "VAL-025" "No features/ directory or no BEH IDs"
fi

# ─── VAL-026: Every FEAT referenced by a capability ─────────────────────────

if has_dir "$SPEC_DIR/capabilities" && [[ -s "$all_feat" ]]; then
  while IFS= read -r feat_id; do
    if ! grep -qx "$feat_id" "$referenced_feat" 2>/dev/null; then
      emit "VAL-026" "warning" "features/" "Orphan feature: $feat_id not referenced by any capability's features[] list"
    fi
  done < "$all_feat"
else
  skip_rule "VAL-026" "No capabilities/ directory or no FEAT IDs"
fi

# ─── VAL-027: Every INV referenced by a behavior ────────────────────────────

if [[ -s "$all_inv" ]]; then
  while IFS= read -r inv_id; do
    if ! grep -qx "$inv_id" "$referenced_inv" 2>/dev/null; then
      emit "VAL-027" "warning" "invariants/" "Orphan invariant: $inv_id not referenced by any behavior's invariants[] list"
    fi
  done < "$all_inv"
else
  skip_rule "VAL-027" "No invariant IDs"
fi

# ─── VAL-028: Every ADR referenced by a behavior ────────────────────────────

if [[ -s "$all_adr" ]]; then
  while IFS= read -r adr_id; do
    if ! grep -qx "$adr_id" "$referenced_adr" 2>/dev/null; then
      emit "VAL-028" "warning" "decisions/" "Orphan ADR: $adr_id not referenced by any behavior's adrs[] list"
    fi
  done < "$all_adr"
else
  skip_rule "VAL-028" "No ADR IDs"
fi

# ─── VAL-029: Every type domain referenced by a behavior ────────────────────

if [[ -s "$all_types" ]]; then
  while IFS= read -r type_domain; do
    if ! grep -qx "$type_domain" "$referenced_types" 2>/dev/null; then
      emit "VAL-029" "info" "types/" "Type domain '$type_domain' not referenced by any behavior's types[] list"
    fi
  done < "$all_types"
else
  skip_rule "VAL-029" "No type domains"
fi

# ─── VAL-030: Every BEH file has invariants or adrs ─────────────────────────

if has_dir "$SPEC_DIR/behaviors"; then
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="behaviors/$(basename "$f")"

    invs=$(get_fm_list "$f" "invariants")
    adrs=$(get_fm_list "$f" "adrs")

    if [[ -z "$invs" && -z "$adrs" ]]; then
      emit "VAL-030" "warning" "$rel" "Behavior file has no invariants[] and no adrs[] — no traceability anchor"
    fi
  done
else
  skip_rule "VAL-030" "No behaviors/ directory"
fi

# ─── VAL-054: Every UX referenced by a deliverable ─────────────────────────

# All UX IDs
all_ux=$(make_temp)
if has_dir "$SPEC_DIR/capabilities"; then
  for f in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    uid=$(get_fm_field "$f" "id")
    [[ -n "$uid" ]] && echo "$uid"
  done | sort -u > "$all_ux"
fi

# UX IDs referenced by deliverables (capabilities[] field)
referenced_ux=$(make_temp)
if has_dir "$SPEC_DIR/deliverables"; then
  for f in "$SPEC_DIR/deliverables"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "capabilities"
  done | sort -u > "$referenced_ux"
fi

if has_dir "$SPEC_DIR/deliverables" && [[ -s "$all_ux" ]]; then
  while IFS= read -r ux_id; do
    if ! grep -qx "$ux_id" "$referenced_ux" 2>/dev/null; then
      emit "VAL-054" "warning" "capabilities/" "Orphan capability: $ux_id not referenced by any deliverable's capabilities[] list"
    fi
  done < "$all_ux"
else
  skip_rule "VAL-054" "No deliverables/ directory or no UX IDs"
fi

# ─── VAL-055: Every FEAT implemented by a library ──────────────────────────

# FEAT IDs referenced by libraries (features[] field)
referenced_feat_by_lib=$(make_temp)
if has_dir "$SPEC_DIR/libraries"; then
  for f in "$SPEC_DIR/libraries"/*/*.md "$SPEC_DIR/libraries"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "features"
  done | sort -u > "$referenced_feat_by_lib"
fi

if has_dir "$SPEC_DIR/libraries" && [[ -s "$all_feat" ]]; then
  while IFS= read -r feat_id; do
    if ! grep -qx "$feat_id" "$referenced_feat_by_lib" 2>/dev/null; then
      emit "VAL-055" "warning" "features/" "Orphan feature: $feat_id not referenced by any library's features[] list"
    fi
  done < "$all_feat"
else
  skip_rule "VAL-055" "No libraries/ directory or no FEAT IDs"
fi

finalize "04-reverse-coverage (VAL-025..030, VAL-054..055)"
