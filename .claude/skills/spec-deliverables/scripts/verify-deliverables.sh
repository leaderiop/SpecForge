#!/usr/bin/env bash
# verify-deliverables.sh — CHK-DLV-001 through CHK-DLV-005
# Checks: deliverable cross-references to capabilities, libraries, and roadmap
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

# ─── Graceful skip if deliverables/ absent ───────────────────────────────────

if [[ ! -d "$SPEC_DIR/deliverables" ]]; then
  echo "info: No deliverables/ directory in $SPEC_DIR — skipping all checks" >&2
  exit 0
fi

# ─── Collect existing UX IDs from capabilities/ ─────────────────────────────

ux_ids=$(make_temp)
if has_dir "$SPEC_DIR/capabilities"; then
  for f in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    uid=$(get_fm_field "$f" "id")
    [[ -n "$uid" ]] && echo "$uid"
  done | sort -u > "$ux_ids"
fi

# ─── Collect existing LIB IDs from libraries/ ───────────────────────────────

lib_ids=$(make_temp)
if has_dir "$SPEC_DIR/libraries"; then
  for f in "$SPEC_DIR/libraries"/*/*.md "$SPEC_DIR/libraries"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    lid=$(get_fm_field "$f" "id")
    [[ -n "$lid" ]] && echo "$lid"
  done | sort -u > "$lib_ids"
fi

# ─── Collect roadmap phase IDs from roadmap/ ─────────────────────────────────

roadmap_ids=$(make_temp)
if has_dir "$SPEC_DIR/roadmap"; then
  for f in "$SPEC_DIR/roadmap"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    echo "$basename_f" | grep -Eo 'RM-[0-9]+' >> "$roadmap_ids" 2>/dev/null || true
    grep -Eo 'RM-[0-9]+' "$f" >> "$roadmap_ids" 2>/dev/null || true
  done
  sort -u -o "$roadmap_ids" "$roadmap_ids"
fi

# ─── Iterate over deliverable files ─────────────────────────────────────────

for f in "$SPEC_DIR/deliverables"/*.md; do
  [[ -f "$f" ]] || continue
  basename_f=$(basename "$f")
  [[ "$basename_f" == "index.md" ]] && continue
  rel="deliverables/$basename_f"

  # ── CHK-DLV-001: capabilities[] → existing UX ──
  if has_dir "$SPEC_DIR/capabilities"; then
    while IFS= read -r cap_ref; do
      [[ -z "$cap_ref" ]] && continue
      if ! grep -qx "$cap_ref" "$ux_ids" 2>/dev/null; then
        emit "CHK-DLV-001" "error" "$rel" "capabilities ref '$cap_ref' does not match any capability in capabilities/"
      fi
    done < <(get_fm_list "$f" "capabilities")
  else
    skip_rule "CHK-DLV-001" "No capabilities/ directory for UX resolution"
  fi

  # ── CHK-DLV-002: depends_on[] → existing LIB ──
  if has_dir "$SPEC_DIR/libraries"; then
    while IFS= read -r lib_ref; do
      [[ -z "$lib_ref" ]] && continue
      if ! grep -qx "$lib_ref" "$lib_ids" 2>/dev/null; then
        emit "CHK-DLV-002" "error" "$rel" "depends_on ref '$lib_ref' does not match any library in libraries/"
      fi
    done < <(get_fm_list "$f" "depends_on")
  else
    skip_rule "CHK-DLV-002" "No libraries/ directory for LIB resolution"
  fi

  # ── CHK-DLV-003: roadmap_releases[] → existing RM ──
  if [[ -s "$roadmap_ids" ]]; then
    while IFS= read -r rm_ref; do
      [[ -z "$rm_ref" ]] && continue
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
        emit "CHK-DLV-003" "warning" "$rel" "roadmap_releases ref '$rm_ref' not found in roadmap/"
      fi
    done < <(get_fm_list "$f" "roadmap_releases")
  else
    skip_rule "CHK-DLV-003" "No roadmap/ directory or no roadmap phase IDs found"
  fi

  # ── CHK-DLV-004: deliverable_type compatible with capability surfaces ──
  dlv_type=$(get_fm_field "$f" "deliverable_type")
  if [[ -n "$dlv_type" ]] && has_dir "$SPEC_DIR/capabilities"; then
    while IFS= read -r cap_ref; do
      [[ -z "$cap_ref" ]] && continue
      # Find the capability file and check its surface field
      for cap_file in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
        [[ -f "$cap_file" ]] || continue
        cap_id=$(get_fm_field "$cap_file" "id")
        if [[ "$cap_id" == "$cap_ref" ]]; then
          surfaces=$(get_fm_list "$cap_file" "surface")
          if [[ -n "$surfaces" ]]; then
            # Map deliverable_type to expected surface keywords
            compatible=false
            case "$dlv_type" in
              app)       echo "$surfaces" | grep -qiE 'desktop|app|gui' && compatible=true ;;
              cli)       echo "$surfaces" | grep -qiE 'cli|terminal|command' && compatible=true ;;
              service)   echo "$surfaces" | grep -qiE 'api|service|server' && compatible=true ;;
              extension) echo "$surfaces" | grep -qiE 'extension|plugin|vscode|ide' && compatible=true ;;
            esac
            if [[ "$compatible" == "false" ]]; then
              emit "CHK-DLV-004" "warning" "$rel" \
                "deliverable_type '$dlv_type' may be incompatible with capability $cap_ref surfaces: $(echo "$surfaces" | tr '\n' ', ' | sed 's/,$//')"
            fi
          fi
          break
        fi
      done
    done < <(get_fm_list "$f" "capabilities")
  fi

done

# ── CHK-DLV-005: transitive coverage — libraries implement features needed by capabilities ──
# This is an informational check that requires full graph traversal
if has_dir "$SPEC_DIR/capabilities" && has_dir "$SPEC_DIR/libraries" && has_dir "$SPEC_DIR/features"; then
  # Collect all features referenced by capabilities that deliverables reference
  dlv_cap_feats=$(make_temp)
  for f in "$SPEC_DIR/deliverables"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    while IFS= read -r cap_ref; do
      [[ -z "$cap_ref" ]] && continue
      for cap_file in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
        [[ -f "$cap_file" ]] || continue
        cap_id=$(get_fm_field "$cap_file" "id")
        if [[ "$cap_id" == "$cap_ref" ]]; then
          get_fm_list "$cap_file" "features"
          break
        fi
      done
    done < <(get_fm_list "$f" "capabilities")
  done | sort -u > "$dlv_cap_feats"

  # Collect all features covered by libraries
  lib_feats=$(make_temp)
  for f in "$SPEC_DIR/libraries"/*/*.md "$SPEC_DIR/libraries"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "features"
  done | sort -u > "$lib_feats"

  # Find features needed by deliverable capabilities but not covered by any library
  if [[ -s "$dlv_cap_feats" ]]; then
    while IFS= read -r feat; do
      [[ -z "$feat" ]] && continue
      if ! grep -qx "$feat" "$lib_feats" 2>/dev/null; then
        emit "CHK-DLV-005" "info" "deliverables/" \
          "Feature $feat is needed by deliverable capabilities but not covered by any library"
      fi
    done < "$dlv_cap_feats"
  fi
else
  skip_rule "CHK-DLV-005" "Missing capabilities/, libraries/, or features/ directory for transitive coverage check"
fi

finalize "verify-deliverables (CHK-DLV-001..005)"
