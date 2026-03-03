#!/usr/bin/env bash
# 10-semantic-consistency.sh — VAL-051 through VAL-053
# Checks: supersession consistency, deprecated behavior hints, port name documentation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

# ─── VAL-051: Superseded ADR has Superseded status ──────────────────────────

if has_dir "$SPEC_DIR/decisions"; then
  # Build map of ADR ID → status
  adr_status=$(make_temp)
  for f in "$SPEC_DIR/decisions"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    aid=$(get_fm_field "$f" "id")
    astatus=$(get_fm_field "$f" "status")
    [[ -n "$aid" ]] && printf '%s\t%s\n' "$aid" "$astatus" >> "$adr_status"
  done

  # Check each ADR's supersedes list
  for f in "$SPEC_DIR/decisions"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="decisions/$(basename "$f")"
    while IFS= read -r sup_ref; do
      [[ -z "$sup_ref" ]] && continue
      # Look up the superseded ADR's status
      sup_status=$(grep "^${sup_ref}	" "$adr_status" 2>/dev/null | cut -f2)
      if [[ -n "$sup_status" ]]; then
        # Check if status is "Superseded" (case-insensitive)
        if [[ "${sup_status,,}" != "superseded" ]]; then
          emit "VAL-051" "warning" "$rel" \
            "Supersedes $sup_ref, but $sup_ref has status '$sup_status' instead of 'Superseded'"
        fi
      fi
    done < <(get_fm_list "$f" "supersedes")
  done
else
  skip_rule "VAL-051" "No decisions/ directory"
fi

# ─── VAL-052: Deprecated behaviors mention replacement ───────────────────────

if has_dir "$SPEC_DIR/behaviors"; then
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    rel="behaviors/$(basename "$f")"

    bstatus=$(get_fm_field "$f" "status")
    if [[ "${bstatus,,}" == "deprecated" ]]; then
      # Search for replacement mentions in file content
      if ! grep -qEi 'superseded by|replaced by|see BEH-|see ADR-|migrated to' "$f" 2>/dev/null; then
        emit "VAL-052" "info" "$rel" \
          "Behavior file has status 'deprecated' but does not mention a replacement"
      fi
    fi
  done
else
  skip_rule "VAL-052" "No behaviors/ directory"
fi

# ─── VAL-053: Port names in behavior frontmatter appear in docs ─────────────

if has_dir "$SPEC_DIR/behaviors"; then
  # Collect all port names from behavior frontmatter
  all_ports=$(make_temp)
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    get_fm_list "$f" "ports"
  done | sort -u > "$all_ports"

  if [[ -s "$all_ports" ]]; then
    # Read port documentation files
    port_docs=$(make_temp)
    for doc in "$SPEC_DIR/types/ports.md" \
               "$SPEC_DIR/architecture/ports-and-adapters.md" \
               "$SPEC_DIR/types/port-catalog.md"; do
      [[ -f "$doc" ]] && cat "$doc" >> "$port_docs"
    done

    if [[ -s "$port_docs" ]]; then
      while IFS= read -r port_name; do
        [[ -z "$port_name" ]] && continue
        if ! grep -qF "$port_name" "$port_docs" 2>/dev/null; then
          emit "VAL-053" "info" "behaviors/" \
            "Port '$port_name' referenced in behavior frontmatter but not documented in types/ or architecture/"
        fi
      done < "$all_ports"
    else
      skip_rule "VAL-053" "No port documentation files found (types/ports.md, architecture/ports-and-adapters.md)"
    fi
  else
    skip_rule "VAL-053" "No port names found in behavior frontmatter"
  fi
else
  skip_rule "VAL-053" "No behaviors/ directory"
fi

finalize "10-semantic-consistency (VAL-051..053)"
