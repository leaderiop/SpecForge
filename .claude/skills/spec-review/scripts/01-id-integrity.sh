#!/usr/bin/env bash
# 01-id-integrity.sh — VAL-001 through VAL-009
# Checks: duplicate IDs, id_range integrity, filename-prefix matching

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

# ─── Helper: check for duplicates in an ID file ─────────────────────────────

# Usage: check_duplicates <id_file> <rule> <dir_label> <entity_label>
check_duplicates() {
  local id_file="$1" rule="$2" dir_label="$3" entity_label="$4"
  [[ -s "$id_file" ]] || return 0
  local dups
  dups=$(cut -f1 "$id_file" | sort | uniq -d)
  [[ -z "$dups" ]] && return 0
  while IFS= read -r dup_id; do
    [[ -z "$dup_id" ]] && continue
    local files
    files=$(grep "^${dup_id}	" "$id_file" | cut -f2 | sort -u | tr '\n' ', ' | sed 's/,$//' || true)
    emit "$rule" "error" "$dir_label/" "Duplicate $entity_label ID $dup_id in files: $files"
  done <<< "$dups"
}

# ─── VAL-001: No duplicate BEH IDs ──────────────────────────────────────────

if has_dir "$SPEC_DIR/behaviors" || has_dir "$SPEC_DIR/plugins"; then
  beh_map=$(make_temp)
  collect_beh_ids "$SPEC_DIR" "$INFIX" > "$beh_map"
  check_duplicates "$beh_map" "VAL-001" "behaviors" "BEH"
else
  skip_rule "VAL-001" "No behaviors/ or plugins/ directory"
fi

# ─── VAL-002: No duplicate FEAT IDs ─────────────────────────────────────────

if has_dir "$SPEC_DIR/features"; then
  feat_ids=$(make_temp)
  for f in "$SPEC_DIR/features"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    fid=$(get_fm_field "$f" "id")
    [[ -n "$fid" ]] && printf '%s\t%s\n' "$fid" "$basename_f" >> "$feat_ids"
  done
  check_duplicates "$feat_ids" "VAL-002" "features" "FEAT"
else
  skip_rule "VAL-002" "No features/ directory"
fi

# ─── VAL-003: No duplicate UX IDs ───────────────────────────────────────────

if has_dir "$SPEC_DIR/capabilities"; then
  ux_ids=$(make_temp)
  for f in "$SPEC_DIR/capabilities"/*.md "$SPEC_DIR/capabilities"/*/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    uid=$(get_fm_field "$f" "id")
    rel_f="${f#"$SPEC_DIR"/capabilities/}"
    [[ -n "$uid" ]] && printf '%s\t%s\n' "$uid" "$rel_f" >> "$ux_ids"
  done
  check_duplicates "$ux_ids" "VAL-003" "capabilities" "UX"
else
  skip_rule "VAL-003" "No capabilities/ directory"
fi

# ─── VAL-004: No duplicate INV IDs ──────────────────────────────────────────

if has_dir "$SPEC_DIR/invariants"; then
  inv_ids=$(make_temp)
  for f in "$SPEC_DIR/invariants"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    iid=$(get_fm_field "$f" "id")
    [[ -n "$iid" ]] && printf '%s\t%s\n' "$iid" "$basename_f" >> "$inv_ids"
  done
  check_duplicates "$inv_ids" "VAL-004" "invariants" "INV"
else
  skip_rule "VAL-004" "No invariants/ directory"
fi

# ─── VAL-005: No duplicate ADR IDs ──────────────────────────────────────────

if has_dir "$SPEC_DIR/decisions"; then
  adr_ids=$(make_temp)
  for f in "$SPEC_DIR/decisions"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    aid=$(get_fm_field "$f" "id")
    [[ -n "$aid" ]] && printf '%s\t%s\n' "$aid" "$basename_f" >> "$adr_ids"
  done
  check_duplicates "$adr_ids" "VAL-005" "decisions" "ADR"
else
  skip_rule "VAL-005" "No decisions/ directory"
fi

# ─── VAL-006: No duplicate TYPE IDs ─────────────────────────────────────────

if has_dir "$SPEC_DIR/types"; then
  type_ids=$(make_temp)
  for f in "$SPEC_DIR/types"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    tid=$(get_fm_field "$f" "id")
    [[ -n "$tid" ]] && printf '%s\t%s\n' "$tid" "$basename_f" >> "$type_ids"
  done
  check_duplicates "$type_ids" "VAL-006" "types" "TYPE"
else
  skip_rule "VAL-006" "No types/ directory"
fi

# ─── No duplicate DLV IDs ────────────────────────────────────────────────────

if has_dir "$SPEC_DIR/deliverables"; then
  dlv_ids=$(make_temp)
  for f in "$SPEC_DIR/deliverables"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    did=$(get_fm_field "$f" "id")
    [[ -n "$did" ]] && printf '%s\t%s\n' "$did" "$basename_f" >> "$dlv_ids"
  done
  check_duplicates "$dlv_ids" "VAL-001" "deliverables" "DLV"
else
  skip_rule "VAL-001" "No deliverables/ directory (DLV)"
fi

# ─── No duplicate LIB IDs ───────────────────────────────────────────────────

if has_dir "$SPEC_DIR/libraries"; then
  lib_ids=$(make_temp)
  collect_lib_file_ids "$SPEC_DIR/libraries" > "$lib_ids"
  check_duplicates "$lib_ids" "VAL-001" "libraries" "LIB"
else
  skip_rule "VAL-001" "No libraries/ directory (LIB)"
fi

# ─── VAL-007: BEH section IDs within file's id_range ────────────────────────

if has_dir "$SPEC_DIR/behaviors"; then
  # Load gap-fill exceptions from requirement-id-scheme if available
  gap_fill=$(make_temp)
  if [[ -f "$SPEC_DIR/process/requirement-id-scheme.md" ]]; then
    # Extract gap-fill/renumber lines, parse range and filename
    # Handles both -- (double hyphen) and – (en-dash) range separators
    awk '
      /gap-fill|renumber|Gap-fill|Renumber/ {
        line = $0
        # Find a range pattern: digits followed by -- or – followed by digits
        if (match(line, /[0-9]+[-–][-–]?[0-9]+/)) {
          range = substr(line, RSTART, RLENGTH)
          # Find a backtick-quoted .md filename
          if (match(line, /`[^`]+\.md`/)) {
            fname = substr(line, RSTART+1, RLENGTH-2)
            # Parse range: split on - or –
            split(range, parts, /[-–]+/)
            if (parts[1] != "" && parts[2] != "")
              printf "%s\t%s\t%s\n", parts[1]+0, parts[2]+0, fname
          }
        }
      }
    ' "$SPEC_DIR/process/requirement-id-scheme.md" > "$gap_fill"
  fi

  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue

    id_range=$(get_fm_field "$f" "id_range")
    [[ -z "$id_range" ]] && continue

    # Parse id_range: "NNN--NNN"
    range_lo=$(echo "$id_range" | sed -E 's/^"?([0-9]+)[-–]+.*/\1/' | sed 's/^0*//')
    range_hi=$(echo "$id_range" | sed -E 's/.*[-–]+([0-9]+)"?$/\1/' | sed 's/^0*//')
    [[ -z "$range_lo" || -z "$range_hi" ]] && continue

    # Collect BEH IDs from this file (use sed to avoid grep exit code issue)
    beh_nums=$(sed -n "s/^## BEH-${INFIX}-\([0-9]\{1,\}\):.*/\1/p" "$f")
    [[ -z "$beh_nums" ]] && continue

    while IFS= read -r raw_num; do
      num=$(echo "$raw_num" | sed 's/^0*//')
      [[ -z "$num" ]] && continue

      if [[ "$num" -lt "$range_lo" || "$num" -gt "$range_hi" ]]; then
        # Check gap-fill exceptions
        is_exception=false
        if [[ -s "$gap_fill" ]]; then
          while IFS=$'\t' read -r glo ghi gfile; do
            if [[ "$gfile" == "$basename_f" && "$num" -ge "$glo" && "$num" -le "$ghi" ]]; then
              is_exception=true
              break
            fi
          done < "$gap_fill"
        fi

        if [[ "$is_exception" == "false" ]]; then
          emit "VAL-007" "error" "behaviors/$basename_f" \
            "BEH-${INFIX}-$(printf '%03d' "$num") outside id_range $id_range (no gap-fill exception found)"
        fi
      fi
    done <<< "$beh_nums"
  done
else
  skip_rule "VAL-007" "No behaviors/ directory"
fi

# ─── VAL-008: id_range overlap detection ────────────────────────────────────

if has_dir "$SPEC_DIR/behaviors"; then
  ranges=$(make_temp)
  for f in "$SPEC_DIR/behaviors"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue

    id_range=$(get_fm_field "$f" "id_range")
    [[ -z "$id_range" ]] && continue

    range_lo=$(echo "$id_range" | sed -E 's/^"?([0-9]+)[-–]+.*/\1/' | sed 's/^0*//')
    range_hi=$(echo "$id_range" | sed -E 's/.*[-–]+([0-9]+)"?$/\1/' | sed 's/^0*//')
    [[ -z "$range_lo" || -z "$range_hi" ]] && continue

    printf '%s\t%s\t%s\n' "$range_lo" "$range_hi" "$basename_f" >> "$ranges"
  done

  if [[ -s "$ranges" ]]; then
    while IFS=$'\t' read -r lo1 hi1 file1; do
      while IFS=$'\t' read -r lo2 hi2 file2; do
        [[ "$file1" == "$file2" ]] && continue
        [[ "$file1" > "$file2" ]] && continue
        if [[ "$lo1" -le "$hi2" && "$lo2" -le "$hi1" ]]; then
          emit "VAL-008" "error" "behaviors/" \
            "id_range overlap: $file1 ($lo1--$hi1) and $file2 ($lo2--$hi2)"
        fi
      done < "$ranges"
    done < "$ranges"
  fi
else
  skip_rule "VAL-008" "No behaviors/ directory"
fi

# ─── VAL-009: Filename prefix matches frontmatter id ────────────────────────

check_filename_prefix() {
  local dir="$1" pattern="$2" dir_label="$3"
  [[ -d "$dir" ]] || return 0
  for f in "$dir"/*.md; do
    [[ -f "$f" ]] || continue
    local basename_f
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    local fid
    fid=$(get_fm_field "$f" "id")
    [[ -z "$fid" ]] && continue
    local file_prefix
    file_prefix=$(echo "$basename_f" | grep -Eo "$pattern" || true)
    if [[ -n "$file_prefix" && "$file_prefix" != "$fid" ]]; then
      emit "VAL-009" "warning" "$dir_label/$basename_f" \
        "Filename prefix '$file_prefix' does not match frontmatter id '$fid'"
    fi
  done
}

check_filename_prefix "$SPEC_DIR/behaviors" "^BEH-${INFIX}-[0-9]+" "behaviors"
check_filename_prefix "$SPEC_DIR/features" "^FEAT-${INFIX}-[0-9]+" "features"
check_filename_prefix "$SPEC_DIR/capabilities" "^UX-${INFIX}-[0-9]+" "capabilities"
check_filename_prefix "$SPEC_DIR/decisions" '^ADR-[0-9]+' "decisions"
check_filename_prefix "$SPEC_DIR/deliverables" "^DLV-${INFIX}-[0-9]+" "deliverables"

# Capabilities may use optional group sub-folders — check those too
if has_dir "$SPEC_DIR/capabilities"; then
  for sub in "$SPEC_DIR/capabilities"/*/; do
    [[ -d "$sub" ]] || continue
    local_label="capabilities/$(basename "$sub")"
    for f in "$sub"*.md; do
      [[ -f "$f" ]] || continue
      basename_f=$(basename "$f")
      [[ "$basename_f" == "index.md" ]] && continue
      fid=$(get_fm_field "$f" "id")
      [[ -z "$fid" ]] && continue
      file_prefix=$(echo "$basename_f" | grep -Eo "^UX-${INFIX}-[0-9]+" || true)
      if [[ -n "$file_prefix" && "$file_prefix" != "$fid" ]]; then
        emit "VAL-009" "warning" "$local_label/$basename_f" \
          "Filename prefix '$file_prefix' does not match frontmatter id '$fid'"
      fi
    done
  done
fi

# Libraries use sub-folders — check files in each family sub-folder
if has_dir "$SPEC_DIR/libraries"; then
  for sub in "$SPEC_DIR/libraries"/*/; do
    [[ -d "$sub" ]] || continue
    local_label="libraries/$(basename "$sub")"
    for f in "$sub"*.md; do
      [[ -f "$f" ]] || continue
      basename_f=$(basename "$f")
      [[ "$basename_f" == "index.md" ]] && continue
      fid=$(get_fm_field "$f" "id")
      [[ -z "$fid" ]] && continue
      file_prefix=$(echo "$basename_f" | grep -Eo "^LIB-${INFIX}-[0-9]+" || true)
      if [[ -n "$file_prefix" && "$file_prefix" != "$fid" ]]; then
        emit "VAL-009" "warning" "$local_label/$basename_f" \
          "Filename prefix '$file_prefix' does not match frontmatter id '$fid'"
      fi
    done
  done
fi

finalize "01-id-integrity (VAL-001..009)"
