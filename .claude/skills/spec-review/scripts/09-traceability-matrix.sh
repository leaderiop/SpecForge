#!/usr/bin/env bash
# 09-traceability-matrix.sh — VAL-049 through VAL-050
# Checks: INV and ADR IDs appear in traceability/ files

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

if ! has_dir "$SPEC_DIR/traceability"; then
  skip_rule "VAL-049" "No traceability/ directory"
  skip_rule "VAL-050" "No traceability/ directory"
  finalize "09-traceability-matrix (VAL-049..050)"
  exit 0
fi

# Read all traceability content once
trace_content=$(make_temp)
cat "$SPEC_DIR/traceability"/*.md > "$trace_content" 2>/dev/null || true

if [[ ! -s "$trace_content" ]]; then
  skip_rule "VAL-049" "No .md files in traceability/"
  skip_rule "VAL-050" "No .md files in traceability/"
  finalize "09-traceability-matrix (VAL-049..050)"
  exit 0
fi

# ─── VAL-049: Every INV ID in traceability index ────────────────────────────

if has_dir "$SPEC_DIR/invariants"; then
  for f in "$SPEC_DIR/invariants"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    iid=$(get_fm_field "$f" "id")
    [[ -z "$iid" ]] && continue
    if ! grep -qF "$iid" "$trace_content" 2>/dev/null; then
      emit "VAL-049" "warning" "invariants/$(basename "$f")" \
        "$iid not found in traceability/ files"
    fi
  done
else
  skip_rule "VAL-049" "No invariants/ directory"
fi

# ─── VAL-050: Every ADR ID in traceability index ────────────────────────────

if has_dir "$SPEC_DIR/decisions"; then
  for f in "$SPEC_DIR/decisions"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    aid=$(get_fm_field "$f" "id")
    [[ -z "$aid" ]] && continue
    if ! grep -qF "$aid" "$trace_content" 2>/dev/null; then
      emit "VAL-050" "warning" "decisions/$(basename "$f")" \
        "$aid not found in traceability/ files"
    fi
  done
else
  skip_rule "VAL-050" "No decisions/ directory"
fi

finalize "09-traceability-matrix (VAL-049..050)"
