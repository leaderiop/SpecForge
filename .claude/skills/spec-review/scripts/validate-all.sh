#!/usr/bin/env bash
# validate-all.sh — Runner for spec-review validation scripts
# Usage: bash validate-all.sh <spec-dir> [--strict] [--only=01,03] [--format=tsv|jsonl]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ─── Parse arguments ─────────────────────────────────────────────────────────

SPEC_DIR=""
STRICT=false
ONLY=""
FORMAT="tsv"

for arg in "$@"; do
  case "$arg" in
    --strict)       STRICT=true ;;
    --only=*)       ONLY="${arg#--only=}" ;;
    --format=tsv)   FORMAT="tsv" ;;
    --format=jsonl) FORMAT="jsonl" ;;
    --format=*)     echo "ERROR: Unknown format: ${arg#--format=}" >&2; exit 2 ;;
    --*)            echo "ERROR: Unknown flag: $arg" >&2; exit 2 ;;
    *)              SPEC_DIR="$arg" ;;
  esac
done

if [[ -z "$SPEC_DIR" ]]; then
  echo "Usage: $0 <spec-dir> [--strict] [--only=01,03] [--format=tsv|jsonl]" >&2
  echo "" >&2
  echo "Flags:" >&2
  echo "  --strict         Treat warnings as errors (exit 1 if any warnings)" >&2
  echo "  --only=01,03     Run only the specified scripts (comma-separated numbers)" >&2
  echo "  --format=tsv     Output format: tsv (default) or jsonl" >&2
  exit 2
fi

# Resolve spec directory
if [[ "$SPEC_DIR" != /* ]]; then
  SPEC_DIR="$(cd "$SPEC_DIR" 2>/dev/null && pwd)" || {
    echo "ERROR: Cannot resolve spec directory: $SPEC_DIR" >&2
    exit 2
  }
fi

if [[ ! -d "$SPEC_DIR" ]]; then
  echo "ERROR: Not a directory: $SPEC_DIR" >&2
  exit 2
fi

# ─── Discover scripts ────────────────────────────────────────────────────────

SCRIPTS=()
for script in "$SCRIPT_DIR"/[0-9][0-9]-*.sh; do
  [[ -f "$script" ]] || continue
  SCRIPTS+=("$script")
done

if [[ ${#SCRIPTS[@]} -eq 0 ]]; then
  echo "ERROR: No validation scripts found in $SCRIPT_DIR" >&2
  exit 2
fi

# Filter by --only if specified
if [[ -n "$ONLY" ]]; then
  FILTERED=()
  IFS=',' read -ra NUMS <<< "$ONLY"
  for script in "${SCRIPTS[@]}"; do
    local_name=$(basename "$script")
    script_num="${local_name%%-*}"
    for num in "${NUMS[@]}"; do
      # Zero-pad the filter number
      padded=$(printf '%02d' "$num")
      if [[ "$script_num" == "$padded" ]]; then
        FILTERED+=("$script")
        break
      fi
    done
  done
  SCRIPTS=("${FILTERED[@]}")
fi

# ─── Phase name mapping ─────────────────────────────────────────────────────

phase_name() {
  case "$1" in
    01) echo "ID Integrity" ;;
    02) echo "Frontmatter Schema" ;;
    03) echo "Forward References" ;;
    04) echo "Reverse Coverage" ;;
    05) echo "Index Completeness" ;;
    06) echo "Overview Completeness" ;;
    07) echo "Link Integrity" ;;
    08) echo "Content Structure" ;;
    09) echo "Traceability Matrix" ;;
    10) echo "Semantic Consistency" ;;
    *)  echo "Unknown" ;;
  esac
}

# ─── Run scripts ─────────────────────────────────────────────────────────────

TOTAL_ERRORS=0
TOTAL_WARNINGS=0
TOTAL_INFOS=0
TOTAL_SKIPS=0
ALL_FINDINGS=""
PHASE_RESULTS=""
HAS_FAILURE=false

echo "=== Spec Validation: $SPEC_DIR ===" >&2
echo "" >&2

for script in "${SCRIPTS[@]}"; do
  script_name=$(basename "$script")
  script_num="${script_name%%-*}"
  pname=$(phase_name "$script_num")

  echo "Running $script_name ($pname)..." >&2

  # Capture output, allow non-zero exit
  local_output=$(bash "$script" "$SPEC_DIR" 2>/dev/null || true)

  # Count findings from this script
  local_errors=0
  local_warnings=0
  local_infos=0
  local_skips=0

  if [[ -n "$local_output" ]]; then
    local_errors=$(echo "$local_output" | grep -c $'^\S.*\terror\t' || true)
    local_warnings=$(echo "$local_output" | grep -c $'^\S.*\twarning\t' || true)
    local_infos=$(echo "$local_output" | grep -c $'^\S.*\tinfo\t' || true)
    local_skips=$(echo "$local_output" | grep -c $'^\S.*\tskip\t' || true)
  fi

  TOTAL_ERRORS=$((TOTAL_ERRORS + local_errors))
  TOTAL_WARNINGS=$((TOTAL_WARNINGS + local_warnings))
  TOTAL_INFOS=$((TOTAL_INFOS + local_infos))
  TOTAL_SKIPS=$((TOTAL_SKIPS + local_skips))

  # Determine phase status
  local_status="PASS"
  if [[ "$local_errors" -gt 0 ]]; then
    local_status="FAIL"
    HAS_FAILURE=true
  elif [[ "$local_warnings" -gt 0 ]]; then
    local_status="WARN"
    if [[ "$STRICT" == "true" ]]; then
      HAS_FAILURE=true
    fi
  fi

  local_detail="${local_errors}E ${local_warnings}W ${local_infos}I"
  if [[ "$local_skips" -gt 0 ]]; then
    local_detail="$local_detail ${local_skips}S"
  fi

  PHASE_RESULTS="${PHASE_RESULTS}${script_num}\t${pname}\t${local_status}\t${local_detail}\n"

  # Accumulate non-skip findings
  if [[ -n "$local_output" ]]; then
    findings=$(echo "$local_output" | grep -v $'\tskip\t' || true)
    if [[ -n "$findings" ]]; then
      ALL_FINDINGS="${ALL_FINDINGS}${findings}"$'\n'
    fi
  fi

  echo "  $local_status ($local_detail)" >&2
done

# ─── Output findings ────────────────────────────────────────────────────────

if [[ -n "$ALL_FINDINGS" ]]; then
  findings_trimmed=$(echo "$ALL_FINDINGS" | sed '/^$/d')
  if [[ "$FORMAT" == "jsonl" ]]; then
    echo "$findings_trimmed" | while IFS=$'\t' read -r rule severity file message; do
      [[ -z "$rule" ]] && continue
      printf '{"rule":"%s","severity":"%s","file":"%s","message":"%s"}\n' \
        "$rule" "$severity" "$file" "$(echo "$message" | sed 's/"/\\"/g')"
    done
  else
    echo "$findings_trimmed"
  fi
fi

# ─── Summary table ───────────────────────────────────────────────────────────

echo "" >&2
echo "╔══════════════════════════════════════════════════════════════╗" >&2
echo "║                    VALIDATION SUMMARY                       ║" >&2
echo "╠════╤═══════════════════════╤════════╤═══════════════════════╣" >&2
echo "║ ## │ Phase                 │ Status │ Findings              ║" >&2
echo "╟────┼───────────────────────┼────────┼───────────────────────╢" >&2
printf "$PHASE_RESULTS" | while IFS=$'\t' read -r num name status detail; do
  [[ -z "$num" ]] && continue
  printf '║ %s │ %-21s │ %-6s │ %-21s ║\n' "$num" "$name" "$status" "$detail" >&2
done
echo "╠════╧═══════════════════════╧════════╧═══════════════════════╣" >&2
TOTAL=$((TOTAL_ERRORS + TOTAL_WARNINGS + TOTAL_INFOS))
printf "║  Totals: %dE  %dW  %dI  (%d findings, %d skipped)        ║\n" \
  "$TOTAL_ERRORS" "$TOTAL_WARNINGS" "$TOTAL_INFOS" "$TOTAL" "$TOTAL_SKIPS" >&2
echo "╚══════════════════════════════════════════════════════════════╝" >&2

# ─── Exit code ───────────────────────────────────────────────────────────────

if [[ "$HAS_FAILURE" == "true" ]]; then
  exit 1
fi
exit 0
