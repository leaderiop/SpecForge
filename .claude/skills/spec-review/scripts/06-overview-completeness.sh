#!/usr/bin/env bash
# 06-overview-completeness.sh — VAL-038 through VAL-042
# Checks: overview.md references all major entity files

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

OVERVIEW="$SPEC_DIR/overview.md"
if [[ ! -f "$OVERVIEW" ]]; then
  skip_rule "VAL-038" "No overview.md"
  skip_rule "VAL-039" "No overview.md"
  skip_rule "VAL-040" "No overview.md"
  skip_rule "VAL-041" "No overview.md"
  skip_rule "VAL-042" "No overview.md"
  finalize "06-overview-completeness (VAL-038..042)"
  exit 0
fi

# Read overview once
overview_content=$(cat "$OVERVIEW")

# Helper: check if a file/ID is mentioned in overview
mentioned_in_overview() {
  local filename="$1"
  local entity_id="$2"
  if echo "$overview_content" | grep -qF "$filename" 2>/dev/null; then
    return 0
  fi
  if [[ -n "$entity_id" ]] && echo "$overview_content" | grep -qF "$entity_id" 2>/dev/null; then
    return 0
  fi
  return 1
}

# ─── VAL-038: Behavior files in overview.md ──────────────────────────────────

if has_dir "$SPEC_DIR/behaviors"; then
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    fid=$(get_fm_field "$f" "id")
    if ! mentioned_in_overview "$basename_f" "$fid"; then
      emit "VAL-038" "warning" "behaviors/$basename_f" "Not referenced in overview.md"
    fi
  done
else
  skip_rule "VAL-038" "No behaviors/ directory"
fi

# ─── VAL-039: ADRs in overview.md ───────────────────────────────────────────

if has_dir "$SPEC_DIR/decisions"; then
  for f in "$SPEC_DIR/decisions"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    fid=$(get_fm_field "$f" "id")
    if ! mentioned_in_overview "$basename_f" "$fid"; then
      emit "VAL-039" "warning" "decisions/$basename_f" "Not referenced in overview.md"
    fi
  done
else
  skip_rule "VAL-039" "No decisions/ directory"
fi

# ─── VAL-040: Type files in overview.md ──────────────────────────────────────

if has_dir "$SPEC_DIR/types"; then
  for f in "$SPEC_DIR/types"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    fid=$(get_fm_field "$f" "id")
    if ! mentioned_in_overview "$basename_f" "$fid"; then
      emit "VAL-040" "info" "types/$basename_f" "Not referenced in overview.md"
    fi
  done
else
  skip_rule "VAL-040" "No types/ directory"
fi

# ─── VAL-041: Architecture files in overview.md ─────────────────────────────

if has_dir "$SPEC_DIR/architecture"; then
  for f in "$SPEC_DIR/architecture"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    if ! mentioned_in_overview "$basename_f" ""; then
      emit "VAL-041" "info" "architecture/$basename_f" "Not referenced in overview.md"
    fi
  done
else
  skip_rule "VAL-041" "No architecture/ directory"
fi

# ─── VAL-042: Feature files in overview.md ───────────────────────────────────

if has_dir "$SPEC_DIR/features"; then
  for f in "$SPEC_DIR/features"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    fid=$(get_fm_field "$f" "id")
    if ! mentioned_in_overview "$basename_f" "$fid"; then
      emit "VAL-042" "info" "features/$basename_f" "Not referenced in overview.md"
    fi
  done
else
  skip_rule "VAL-042" "No features/ directory"
fi

finalize "06-overview-completeness (VAL-038..042)"
