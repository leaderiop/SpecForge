#!/usr/bin/env bash
# View Spec Verify Script
# Validates cross-references across all YAML entity files.
# Run from the spec root directory (where index.yaml lives).
#
# Usage: ./scripts/verify.sh
# Exit code: 0 = all checks pass, 1 = one or more checks fail

set -euo pipefail

SPEC_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PASS_COUNT=0
FAIL_COUNT=0
RESULTS=()

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  RESULTS+=("| $1 | $2 | PASS | $3 |")
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  RESULTS+=("| $1 | $2 | **FAIL** | $3 |")
}

# Collect all YAML entity files (exclude index.yaml)
YAML_FILES=()
while IFS= read -r f; do
  YAML_FILES+=("$f")
done < <(find "$SPEC_DIR" -name '*.yaml' -not -name 'index.yaml' | sort)

if [ ${#YAML_FILES[@]} -eq 0 ]; then
  echo "No YAML files found in $SPEC_DIR"
  exit 1
fi

# Extract all IDs
ALL_IDS=()
declare -A ID_FILE_MAP
for f in "${YAML_FILES[@]}"; do
  while IFS= read -r id; do
    ALL_IDS+=("$id")
    ID_FILE_MAP["$id"]="$f"
  done < <(grep -E '^id:\s+' "$f" | sed 's/^id:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ')
done

# ─── CHECK 1: ID Uniqueness ───
declare -A ID_COUNT
DUPES=""
for id in "${ALL_IDS[@]}"; do
  ID_COUNT["$id"]=$(( ${ID_COUNT["$id"]:-0} + 1 ))
done
for id in "${!ID_COUNT[@]}"; do
  if [ "${ID_COUNT[$id]}" -gt 1 ]; then
    DUPES="$DUPES $id"
  fi
done

if [ -z "$DUPES" ]; then
  pass "1" "ID Uniqueness" "${#ALL_IDS[@]} IDs, all unique"
else
  fail "1" "ID Uniqueness" "Duplicates:$DUPES"
fi

# ─── CHECK 2: Ref Resolution ───
ALL_REFS=()
BROKEN_REFS=""
for f in "${YAML_FILES[@]}"; do
  while IFS= read -r ref; do
    ref_clean=$(echo "$ref" | sed 's/.*\$ref:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ' | sed 's/}.*//')
    if [[ "$ref_clean" =~ ^(WF|PG|CMP|ELM|ACT|EVT|STR)- ]]; then
      ALL_REFS+=("$ref_clean")
      FOUND=0
      for id in "${ALL_IDS[@]}"; do
        if [ "$id" = "$ref_clean" ]; then
          FOUND=1
          break
        fi
      done
      if [ "$FOUND" -eq 0 ]; then
        BROKEN_REFS="$BROKEN_REFS $ref_clean"
      fi
    fi
  done < <(grep -E '\$ref:\s*' "$f" || true)
done

if [ -z "$BROKEN_REFS" ]; then
  pass "2" "Ref Resolution" "${#ALL_REFS[@]} refs, all resolved"
else
  fail "2" "Ref Resolution" "Broken:$BROKEN_REFS"
fi

# ─── CHECK 3: No Orphans ───
ORPHANS=""
for id in "${ALL_IDS[@]}"; do
  # Wireframe roots are exempt
  if [[ "$id" == WF-* ]]; then
    continue
  fi
  FOUND=0
  for f in "${YAML_FILES[@]}"; do
    if grep -q "$id" "$f" 2>/dev/null; then
      FILE_ID=$(grep -E '^id:\s+' "$f" | sed 's/^id:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ')
      if [ "$FILE_ID" != "$id" ]; then
        FOUND=1
        break
      fi
    fi
  done
  # Also check index.yaml
  if [ "$FOUND" -eq 0 ] && [ -f "$SPEC_DIR/index.yaml" ]; then
    if grep -q "$id" "$SPEC_DIR/index.yaml" 2>/dev/null; then
      FOUND=1
    fi
  fi
  if [ "$FOUND" -eq 0 ]; then
    ORPHANS="$ORPHANS $id"
  fi
done

if [ -z "$ORPHANS" ]; then
  pass "3" "No Orphans" "All non-root entities referenced"
else
  fail "3" "No Orphans" "Orphans:$ORPHANS"
fi

# ─── CHECK 4: Flux Cycle Complete ───
FLUX_ISSUES=""

# Check: Actions must have events-dispatched
for f in "${YAML_FILES[@]}"; do
  ENTITY=$(grep -E '^entity:\s+' "$f" 2>/dev/null | sed 's/^entity:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ')
  FILE_ID=$(grep -E '^id:\s+' "$f" 2>/dev/null | sed 's/^id:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ')
  if [ "$ENTITY" = "action" ]; then
    if ! grep -q 'events-dispatched' "$f" 2>/dev/null; then
      FLUX_ISSUES="$FLUX_ISSUES $FILE_ID(no-events)"
    fi
  fi
  if [ "$ENTITY" = "event" ]; then
    if ! grep -q 'target-stores' "$f" 2>/dev/null; then
      FLUX_ISSUES="$FLUX_ISSUES $FILE_ID(no-stores)"
    fi
  fi
  if [ "$ENTITY" = "element" ]; then
    if grep -q 'actions:' "$f" 2>/dev/null; then
      if ! grep -qE '\$ref:\s*ACT-' "$f" 2>/dev/null; then
        FLUX_ISSUES="$FLUX_ISSUES $FILE_ID(no-action-ref)"
      fi
    fi
  fi
done

if [ -z "$FLUX_ISSUES" ]; then
  pass "4" "Flux Cycle Complete" "Actions→Events, Events→Stores, Elements→Actions"
else
  fail "4" "Flux Cycle Complete" "Issues:$FLUX_ISSUES"
fi

# ─── CHECK 5: Docs Existence ───
MISSING_DOCS=""
for f in "${YAML_FILES[@]}"; do
  DOC_PATH=$(grep -E '^docs:\s+' "$f" 2>/dev/null | sed 's/^docs:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ')
  if [ -n "$DOC_PATH" ]; then
    DIR=$(dirname "$f")
    RESOLVED="$DIR/$DOC_PATH"
    if [ ! -f "$RESOLVED" ]; then
      FILE_ID=$(grep -E '^id:\s+' "$f" | sed 's/^id:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ')
      MISSING_DOCS="$MISSING_DOCS $FILE_ID($DOC_PATH)"
    fi
  fi
done

if [ -z "$MISSING_DOCS" ]; then
  pass "5" "Docs Existence" "All docs paths resolve"
else
  fail "5" "Docs Existence" "Missing:$MISSING_DOCS"
fi

# ─── CHECK 6: Tests Existence ───
MISSING_TESTS=""
for f in "${YAML_FILES[@]}"; do
  TEST_PATH=$(grep -E '^tests:\s+' "$f" 2>/dev/null | sed 's/^tests:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ')
  if [ -n "$TEST_PATH" ]; then
    DIR=$(dirname "$f")
    RESOLVED="$DIR/$TEST_PATH"
    if [ ! -f "$RESOLVED" ]; then
      FILE_ID=$(grep -E '^id:\s+' "$f" | sed 's/^id:\s*//' | tr -d '"' | tr -d "'" | tr -d ' ')
      MISSING_TESTS="$MISSING_TESTS $FILE_ID($TEST_PATH)"
    fi
  fi
done

if [ -z "$MISSING_TESTS" ]; then
  pass "6" "Tests Existence" "All tests paths resolve"
else
  fail "6" "Tests Existence" "Missing:$MISSING_TESTS"
fi

# ─── CHECK 7: Index Completeness ───
INDEX_FILE="$SPEC_DIR/index.yaml"
MISSING_INDEX=""
if [ -f "$INDEX_FILE" ]; then
  for f in "${YAML_FILES[@]}"; do
    REL_PATH="${f#"$SPEC_DIR"/}"
    BASENAME=$(basename "$REL_PATH")
    if ! grep -q "$BASENAME\|$REL_PATH" "$INDEX_FILE" 2>/dev/null; then
      MISSING_INDEX="$MISSING_INDEX $REL_PATH"
    fi
  done
  if [ -z "$MISSING_INDEX" ]; then
    pass "7" "Index Completeness" "All YAML files referenced in index"
  else
    fail "7" "Index Completeness" "Missing from index:$MISSING_INDEX"
  fi
else
  fail "7" "Index Completeness" "index.yaml not found"
fi

# ─── OUTPUT ───
echo ""
echo "## Verify Results"
echo ""
echo "| # | Check | Status | Detail |"
echo "|---|-------|--------|--------|"
for r in "${RESULTS[@]}"; do
  echo "$r"
done
echo ""
echo "**$PASS_COUNT passed, $FAIL_COUNT failed** out of 7 checks."
echo ""

if [ "$FAIL_COUNT" -gt 0 ]; then
  exit 1
fi
exit 0
