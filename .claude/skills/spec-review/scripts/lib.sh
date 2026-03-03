#!/usr/bin/env bash
# lib.sh — Shared utilities for spec-review validation scripts
# Sourced by all NN-*.sh scripts. Not executable on its own.

set -euo pipefail

# ─── Temp file management ────────────────────────────────────────────────────

_TMPFILES=()

make_temp() {
  local tmp
  tmp=$(mktemp "${TMPDIR:-/tmp}/specval.XXXXXX")
  _TMPFILES+=("$tmp")
  echo "$tmp"
}

cleanup_temps() {
  for f in "${_TMPFILES[@]+"${_TMPFILES[@]}"}"; do
    rm -f "$f"
  done
}
trap cleanup_temps EXIT

# ─── Findings file ───────────────────────────────────────────────────────────
# All findings are appended to FINDINGS_FILE. At the end, finalize() dumps
# them to stdout and computes summary. This avoids subshell counter issues.

FINDINGS_FILE=$(mktemp "${TMPDIR:-/tmp}/specval-findings.XXXXXX")
_TMPFILES+=("$FINDINGS_FILE")

# TSV output: RULE\tSEVERITY\tFILE\tMESSAGE
emit() {
  local rule="$1" severity="$2" file="$3" message="$4"
  printf '%s\t%s\t%s\t%s\n' "$rule" "$severity" "$file" "$message" >> "$FINDINGS_FILE"
}

skip_rule() {
  local rule="$1" reason="$2"
  printf '%s\tskip\t-\t%s\n' "$rule" "$reason" >> "$FINDINGS_FILE"
}

# Dump findings to stdout, summary to stderr, return exit code
finalize() {
  local script_name="$1"

  # Output non-skip findings to stdout
  grep -v $'\tskip\t' "$FINDINGS_FILE" 2>/dev/null || true

  # Count from file (immune to subshell issues)
  local errors warnings infos total
  errors=$(grep -c $'\terror\t' "$FINDINGS_FILE" || true)
  warnings=$(grep -c $'\twarning\t' "$FINDINGS_FILE" || true)
  infos=$(grep -c $'\tinfo\t' "$FINDINGS_FILE" || true)
  # grep -c returns 0 even on no-match; || true prevents pipefail exit
  errors=${errors:-0}
  warnings=${warnings:-0}
  infos=${infos:-0}
  total=$((errors + warnings + infos))

  echo "" >&2
  echo "=== $script_name ===" >&2
  echo "  Errors:   $errors" >&2
  echo "  Warnings: $warnings" >&2
  echo "  Info:     $infos" >&2
  echo "  Total:    $total" >&2

  if [[ "$errors" -gt 0 ]]; then
    return 1
  fi
  return 0
}

# ─── Spec directory resolution ───────────────────────────────────────────────

resolve_spec_dir() {
  local dir="${1:-${SPEC_DIR:-}}"
  if [[ -z "$dir" ]]; then
    echo "Usage: $0 <spec-dir>" >&2
    return 1
  fi
  if [[ "$dir" != /* ]]; then
    dir="$(cd "$dir" 2>/dev/null && pwd)" || {
      echo "ERROR: Cannot resolve spec directory: $dir" >&2
      return 1
    }
  fi
  if [[ ! -d "$dir" ]]; then
    echo "ERROR: Not a directory: $dir" >&2
    return 1
  fi
  echo "$dir"
}

# ─── Infix detection ────────────────────────────────────────────────────────

detect_infix() {
  local spec_dir="$1"
  local infix=""

  # Strategy 1: behaviors/index.yaml infix field
  if [[ -f "$spec_dir/behaviors/index.yaml" ]]; then
    infix=$(grep -E '^infix:' "$spec_dir/behaviors/index.yaml" 2>/dev/null | head -1 | sed 's/^infix:[[:space:]]*//' | tr -d '"' | tr -d "'" || true)
    if [[ -n "$infix" ]]; then
      echo "$infix"
      return 0
    fi
  fi

  # Strategy 2: BEH filename pattern in behaviors/
  if [[ -d "$spec_dir/behaviors" ]]; then
    local beh_file
    beh_file=$(ls "$spec_dir/behaviors/" 2>/dev/null | grep -E '^BEH-[A-Z]+-[0-9]' | head -1 || true)
    if [[ -n "$beh_file" ]]; then
      infix=$(echo "$beh_file" | sed -E 's/^BEH-([A-Z]+)-.*/\1/')
      if [[ -n "$infix" ]]; then
        echo "$infix"
        return 0
      fi
    fi
  fi

  # Strategy 3: any index.yaml with infix field
  local yaml_file
  for yaml_file in "$spec_dir"/*/index.yaml; do
    [[ -f "$yaml_file" ]] || continue
    infix=$(grep -E '^infix:' "$yaml_file" 2>/dev/null | head -1 | sed 's/^infix:[[:space:]]*//' | tr -d '"' | tr -d "'" || true)
    if [[ -n "$infix" ]]; then
      echo "$infix"
      return 0
    fi
  done

  # Strategy 4: path-based derivation
  local dir_basename
  dir_basename=$(basename "$spec_dir")
  case "$dir_basename" in
    specforge) echo "SF" ;;
    guard)     echo "GD" ;;
    flow)      echo "FL" ;;
    saga)      echo "SG" ;;
    crypto)    echo "CR" ;;
    *)
      echo "$dir_basename" | tr '[:lower:]' '[:upper:]' | cut -c1-2
      ;;
  esac
}

# ─── Legacy format detection ────────────────────────────────────────────────

detect_legacy_format() {
  local spec_dir="$1"
  local count
  count=$(ls "$spec_dir" 2>/dev/null | grep -cE '^[0-9]{2}-.*\.md$' || echo 0)
  if [[ "$count" -gt 2 ]]; then
    echo "true"
  else
    echo "false"
  fi
}

# ─── Frontmatter extraction ─────────────────────────────────────────────────

has_frontmatter() {
  local file="$1"
  [[ -f "$file" ]] || return 1
  local first_line
  first_line=$(head -1 "$file")
  [[ "$first_line" == "---" ]]
}

# Extract a scalar frontmatter field value
get_fm_field() {
  local file="$1" field="$2"
  [[ -f "$file" ]] || return 0
  awk -v field="$field" '
    BEGIN { in_fm = 0 }
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; next }
    in_fm && /^---[[:space:]]*$/ { exit }
    in_fm {
      pat = "^" field ":"
      if ($0 ~ pat) {
        sub("^" field ":[[:space:]]*", "")
        gsub(/^["'"'"']|["'"'"']$/, "")
        print
        exit
      }
    }
  ' "$file"
}

# Extract a YAML list field as newline-separated values
# Handles both block style (- item) and flow style ([a, b, c])
get_fm_list() {
  local file="$1" field="$2"
  [[ -f "$file" ]] || return 0
  awk -v field="$field" '
    BEGIN { in_fm = 0; in_list = 0 }
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; next }
    in_fm && /^---[[:space:]]*$/ { exit }
    in_fm {
      pat = "^" field ":"
      if ($0 ~ pat) {
        val = $0
        sub("^" field ":[[:space:]]*", "", val)
        if (val ~ /^\[/) {
          gsub(/[\[\]]/, "", val)
          n = split(val, items, /[[:space:]]*,[[:space:]]*/)
          for (i = 1; i <= n; i++) {
            v = items[i]
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
            gsub(/^["'"'"']|["'"'"']$/, "", v)
            if (v != "") print v
          }
          exit
        }
        if (val ~ /^[[:space:]]*$/) {
          in_list = 1
          next
        }
        gsub(/^["'"'"']|["'"'"']$/, "", val)
        if (val != "") print val
        exit
      }
      if (in_list) {
        if (/^[[:space:]]*-[[:space:]]/) {
          val = $0
          sub(/^[[:space:]]*-[[:space:]]*/, "", val)
          gsub(/^["'"'"']|["'"'"']$/, "", val)
          if (val != "") print val
        } else if (/^[[:space:]]*$/) {
          next
        } else {
          exit
        }
      }
    }
  ' "$file"
}

# ─── Index.yaml file extraction ─────────────────────────────────────────────

# Extract file: values from any index.yaml format (entries/features/groups/families).
# Works because all four formats use a consistent `file:` key for referencing .md files.
# For libraries (families[].libraries[].file), paths include family sub-folder prefixes
# (e.g., "core/LIB-XX-001-di-kernel.md").
extract_index_files() {
  local yaml="$1"
  [[ -f "$yaml" ]] || return 0
  grep -E '^\s+file:\s*' "$yaml" | sed -E 's/^[[:space:]]*file:[[:space:]]*//' | sed 's/^["'"'"']//;s/["'"'"']$//' | sed 's/[[:space:]]*$//' || true
}

# ─── BEH ID collection ──────────────────────────────────────────────────────

# Collect all ## BEH-{INFIX}-NNN: headers from behaviors/ and plugins/
# Output: BEH-XX-NNN\tfilename (one per line)
collect_beh_ids() {
  local spec_dir="$1" infix="$2"
  local dir
  for dir in "$spec_dir/behaviors" "$spec_dir/plugins"; do
    [[ -d "$dir" ]] || continue
    local f
    for f in "$dir"/*.md; do
      [[ -f "$f" ]] || continue
      local file_basename
      file_basename=$(basename "$f")
      # Use sed to extract BEH IDs (avoids grep exit-code issues in pipelines)
      sed -n "s/^## \(BEH-${infix}-[0-9]\{1,\}\):.*/\1/p" "$f" | while IFS= read -r id; do
        printf '%s\t%s\n' "$id" "$file_basename"
      done
    done
  done
}

# ─── Library file ID collection (sub-folder traversal) ───────────────────────

# Collect IDs from libraries/ which uses family sub-folders (libraries/*/*.md)
# Output: ID\trelative_path (one per line)
collect_lib_file_ids() {
  local lib_dir="$1"
  [[ -d "$lib_dir" ]] || return 0
  local f
  for f in "$lib_dir"/*/*.md "$lib_dir"/*.md; do
    [[ -f "$f" ]] || continue
    [[ "$(basename "$f")" == "index.md" ]] && continue
    local lid
    lid=$(get_fm_field "$f" "id")
    [[ -n "$lid" ]] || continue
    local rel
    rel="${f#"$lib_dir"/}"
    printf '%s\t%s\n' "$lid" "$rel"
  done
}

# ─── Directory helpers ───────────────────────────────────────────────────────

list_md_files() {
  local dir="$1"
  [[ -d "$dir" ]] || return 0
  ls "$dir" 2>/dev/null | grep -E '\.md$' | grep -v '^index\.md$' || true
}

has_dir() {
  [[ -d "$1" ]]
}

# ─── Path helpers ────────────────────────────────────────────────────────────

rel_path() {
  local full="$1" base="$2"
  echo "${full#"$base"/}"
}
