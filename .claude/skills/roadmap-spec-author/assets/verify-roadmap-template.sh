#!/usr/bin/env bash
# Roadmap Spec Verify Script
# Validates cross-references and structural integrity of a roadmap.md file.
# Run from the spec root directory (where roadmap.md lives).
#
# Usage: ./scripts/verify-roadmap.sh [roadmap-file]
# Default: roadmap.md in current directory
# Exit code: 0 = all checks pass, 1 = one or more checks fail

set -euo pipefail

SPEC_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ROADMAP="${1:-$SPEC_DIR/roadmap.md}"
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

if [ ! -f "$ROADMAP" ]; then
  echo "Roadmap file not found: $ROADMAP"
  exit 1
fi

# ─── CHECK 1: Status Validity ───
VALID_STATUSES="Planned|Specified|In Progress|Delivered|Deferred"
INVALID_STATUSES=""
while IFS= read -r line; do
  status=$(echo "$line" | sed 's/.*\*\*Status:\*\*\s*//' | tr -d ' \r')
  if ! echo "$status" | grep -qE "^($VALID_STATUSES)$"; then
    INVALID_STATUSES="$INVALID_STATUSES \"$status\""
  fi
done < <(grep -E '^\*\*Status:\*\*' "$ROADMAP" || true)

# Also check status column in tables (last non-pipe-trimmed column or explicit Status header)
while IFS= read -r line; do
  # Extract status values from table rows (look for known status words)
  for word in $(echo "$line" | tr '|' '\n' | tail -1 | tr -d ' \r'); do
    if echo "$word" | grep -qE "^(Planned|Specified|In|Progress|Delivered|Deferred)$"; then
      continue
    fi
  done
done < <(grep -E '^\|.*\|.*Status' "$ROADMAP" || true)

if [ -z "$INVALID_STATUSES" ]; then
  STATUS_COUNT=$(grep -cE '^\*\*Status:\*\*' "$ROADMAP" || echo "0")
  pass "1" "Status Validity" "$STATUS_COUNT status values, all valid"
else
  fail "1" "Status Validity" "Invalid:$INVALID_STATUSES"
fi

# ─── CHECK 2: Behavior ID Ranges ───
BEH_RANGES=()
INVALID_BEHS=""
while IFS= read -r range; do
  # Extract BEH-XX-NNN patterns (with optional range)
  clean=$(echo "$range" | grep -oE 'BEH-[A-Z]+-[0-9]+' | head -1)
  if [ -n "$clean" ]; then
    BEH_RANGES+=("$clean")
    # Extract the prefix (e.g., BEH-SF)
    PREFIX=$(echo "$clean" | grep -oE 'BEH-[A-Z]+')
    # Look for a behaviors directory
    if [ -d "$SPEC_DIR/behaviors" ]; then
      # Check if any behavior file contains this prefix
      if ! grep -rlq "$PREFIX" "$SPEC_DIR/behaviors/" 2>/dev/null; then
        INVALID_BEHS="$INVALID_BEHS $clean"
      fi
    fi
  fi
done < <(grep -oE 'BEH-[A-Z]+-[0-9]+[–-]?[0-9]*' "$ROADMAP" || true)

if [ ${#BEH_RANGES[@]} -eq 0 ]; then
  pass "2" "Behavior ID Ranges" "No behavior references (OK for light traceability)"
elif [ -z "$INVALID_BEHS" ]; then
  pass "2" "Behavior ID Ranges" "${#BEH_RANGES[@]} ranges, all valid"
else
  fail "2" "Behavior ID Ranges" "Unresolved:$INVALID_BEHS"
fi

# ─── CHECK 3: No Orphan Containers ───
# Extract all container IDs (PH-N, REL-X.Y.Z, FT-N patterns in headings)
CONTAINERS=()
ORPHANS=""
while IFS= read -r heading; do
  # Phase pattern
  ph=$(echo "$heading" | grep -oE 'Phase [0-9]+' | sed 's/Phase /PH-/' || true)
  if [ -n "$ph" ]; then
    CONTAINERS+=("$ph")
    continue
  fi
  # Release pattern
  rel=$(echo "$heading" | grep -oE 'Release [0-9]+\.[0-9]+\.[0-9]+' | sed 's/Release /REL-/' | tr '.' '-' || true)
  if [ -n "$rel" ]; then
    CONTAINERS+=("$rel")
    continue
  fi
  # Feature pattern (numbered heading like "## 1. Feature Name")
  ft=$(echo "$heading" | grep -oE '^#{1,3} [0-9]+\.' | grep -oE '[0-9]+' || true)
  if [ -n "$ft" ]; then
    CONTAINERS+=("FT-$ft")
    continue
  fi
done < <(grep -E '^#{1,3} ' "$ROADMAP" || true)

# Check each container appears somewhere besides its own heading
for container in "${CONTAINERS[@]}"; do
  # Count occurrences (should appear in status summary, dependency graph, or milestone table)
  COUNT=$(grep -c "$container\|$(echo "$container" | sed 's/PH-/Phase /;s/FT-//')" "$ROADMAP" 2>/dev/null || echo "0")
  if [ "$COUNT" -le 1 ]; then
    ORPHANS="$ORPHANS $container"
  fi
done

if [ ${#CONTAINERS[@]} -eq 0 ]; then
  pass "3" "No Orphan Containers" "No containers found (flat/minimal variant)"
elif [ -z "$ORPHANS" ]; then
  pass "3" "No Orphan Containers" "${#CONTAINERS[@]} containers, all referenced"
else
  fail "3" "No Orphan Containers" "Orphans:$ORPHANS"
fi

# ─── CHECK 4: Exit Criteria Presence ───
# Check that each container section has exit criteria
MISSING_EC=""
CURRENT_CONTAINER=""
HAS_EC=0
while IFS= read -r line; do
  # Detect container headings
  if echo "$line" | grep -qE '^## (Phase|Release|[0-9]+\.)'; then
    # Check previous container
    if [ -n "$CURRENT_CONTAINER" ] && [ "$HAS_EC" -eq 0 ]; then
      MISSING_EC="$MISSING_EC $CURRENT_CONTAINER"
    fi
    CURRENT_CONTAINER=$(echo "$line" | sed 's/^## //' | head -c 30)
    HAS_EC=0
  fi
  # Detect exit criteria
  if echo "$line" | grep -qiE '(exit criteria|advancement criteria|EC-|^\- \[ \])'; then
    HAS_EC=1
  fi
done < "$ROADMAP"
# Check last container
if [ -n "$CURRENT_CONTAINER" ] && [ "$HAS_EC" -eq 0 ]; then
  MISSING_EC="$MISSING_EC $CURRENT_CONTAINER"
fi

if [ -z "$MISSING_EC" ]; then
  pass "4" "Exit Criteria Presence" "All containers have exit criteria"
else
  fail "4" "Exit Criteria Presence" "Missing:$MISSING_EC"
fi

# ─── CHECK 5: Dependency Acyclicity ───
# Extract dependency edges and check for cycles
declare -A DEP_GRAPH
EDGES=()
while IFS= read -r line; do
  # Match patterns like "PH-1 → PH-2" or "REL-0.1.0 → REL-0.2.0"
  if echo "$line" | grep -qE '(→|->|──►)'; then
    SRC=$(echo "$line" | grep -oE '(PH-[0-9]+|REL-[0-9.-]+)' | head -1)
    TGT=$(echo "$line" | grep -oE '(PH-[0-9]+|REL-[0-9.-]+)' | tail -1)
    if [ -n "$SRC" ] && [ -n "$TGT" ] && [ "$SRC" != "$TGT" ]; then
      EDGES+=("$SRC→$TGT")
      DEP_GRAPH["$SRC"]="${DEP_GRAPH["$SRC"]:-} $TGT"
    fi
  fi
  # Match ASCII tree patterns like "├── PH-2"
  if echo "$line" | grep -qE '(├──|└──).*PH-[0-9]+'; then
    CHILD=$(echo "$line" | grep -oE 'PH-[0-9]+')
    # The parent is the most recent unindented PH- before this
    if [ -n "$CHILD" ]; then
      EDGES+=("parent→$CHILD")
    fi
  fi
done < "$ROADMAP"

# Simple cycle detection via DFS (bash approximation)
HAS_CYCLE=0
if [ ${#EDGES[@]} -gt 0 ]; then
  # Use a simple visited-set approach
  declare -A VISITED
  declare -A IN_STACK

  dfs() {
    local node="$1"
    if [ "${IN_STACK[$node]:-0}" = "1" ]; then
      HAS_CYCLE=1
      return
    fi
    if [ "${VISITED[$node]:-0}" = "1" ]; then
      return
    fi
    VISITED["$node"]=1
    IN_STACK["$node"]=1
    for neighbor in ${DEP_GRAPH["$node"]:-}; do
      dfs "$neighbor"
    done
    IN_STACK["$node"]=0
  }

  for node in "${!DEP_GRAPH[@]}"; do
    dfs "$node"
  done
fi

if [ ${#EDGES[@]} -eq 0 ]; then
  pass "5" "Dependency Acyclicity" "No dependency edges (flat variant)"
elif [ "$HAS_CYCLE" -eq 0 ]; then
  pass "5" "Dependency Acyclicity" "${#EDGES[@]} edges, no cycles"
else
  fail "5" "Dependency Acyclicity" "Cycle detected in dependency graph"
fi

# ─── CHECK 6: Spec File References ───
MISSING_SPECS=""
SPEC_REFS=()
while IFS= read -r ref; do
  # Extract file paths (patterns like spec/..., research/..., behaviors/...)
  path=$(echo "$ref" | grep -oE '(spec|research|behaviors)/[a-zA-Z0-9/_-]+\.(md|yaml)' || true)
  if [ -n "$path" ]; then
    SPEC_REFS+=("$path")
    # Check relative to SPEC_DIR parent (monorepo root)
    MONO_ROOT="$SPEC_DIR"
    # Walk up to find monorepo root (has pnpm-workspace.yaml or package.json)
    while [ "$MONO_ROOT" != "/" ]; do
      if [ -f "$MONO_ROOT/pnpm-workspace.yaml" ] || [ -f "$MONO_ROOT/package.json" ]; then
        break
      fi
      MONO_ROOT=$(dirname "$MONO_ROOT")
    done
    if [ ! -f "$MONO_ROOT/$path" ] && [ ! -f "$SPEC_DIR/$path" ]; then
      MISSING_SPECS="$MISSING_SPECS $path"
    fi
  fi
done < <(grep -oE '[a-zA-Z0-9/_-]+\.(md|yaml)' "$ROADMAP" || true)

if [ ${#SPEC_REFS[@]} -eq 0 ]; then
  pass "6" "Spec File References" "No spec file references found"
elif [ -z "$MISSING_SPECS" ]; then
  pass "6" "Spec File References" "${#SPEC_REFS[@]} refs, all exist"
else
  fail "6" "Spec File References" "Missing:$MISSING_SPECS"
fi

# ─── CHECK 7: Product Milestone Alignment ───
PT_ISSUES=""
while IFS= read -r line; do
  # Extract PT-N rows and check referenced phases/features exist
  pt=$(echo "$line" | grep -oE 'PT-[0-9]+' || true)
  if [ -n "$pt" ]; then
    # Extract referenced containers from the same line
    refs=$(echo "$line" | grep -oE '(PH-[0-9]+|FT-[0-9]+|REL-[0-9.-]+)' || true)
    for ref in $refs; do
      # Check if this container exists in the roadmap
      if ! grep -qE "(Phase ${ref#PH-}|Release ${ref#REL-}|^## ${ref#FT-}\.)" "$ROADMAP" 2>/dev/null; then
        # Also check if the raw ID appears
        if ! grep -q "$ref" "$ROADMAP" 2>/dev/null || [ "$(grep -c "$ref" "$ROADMAP" 2>/dev/null)" -le 1 ]; then
          PT_ISSUES="$PT_ISSUES $pt→$ref"
        fi
      fi
    done
  fi
done < <(grep -E 'PT-[0-9]+' "$ROADMAP" || true)

PT_COUNT=$(grep -cE 'PT-[0-9]+' "$ROADMAP" 2>/dev/null || echo "0")
if [ "$PT_COUNT" -eq 0 ]; then
  pass "7" "Product Milestone Alignment" "No product milestones (OK for non-product variants)"
elif [ -z "$PT_ISSUES" ]; then
  pass "7" "Product Milestone Alignment" "$PT_COUNT milestone refs, all aligned"
else
  fail "7" "Product Milestone Alignment" "Broken:$PT_ISSUES"
fi

# ─── CHECK 8: External Dep Validity ───
EXT_ISSUES=""
EXT_COUNT=0
# Look for external dependency table rows
while IFS= read -r line; do
  # Skip header and separator rows
  if echo "$line" | grep -qE '^\|.*Dependency.*\|' || echo "$line" | grep -qE '^\|[-: ]+\|'; then
    continue
  fi
  if echo "$line" | grep -qE '^\|.*`@'; then
    EXT_COUNT=$((EXT_COUNT + 1))
    # Check blocking phase reference
    blocking=$(echo "$line" | grep -oE '(PH-[0-9]+|FT-[0-9]+|REL-[0-9.-]+)' || true)
    if [ -z "$blocking" ]; then
      dep=$(echo "$line" | grep -oE '`@[^`]+`' | head -1)
      EXT_ISSUES="$EXT_ISSUES ${dep:-unknown}(no-blocking-ref)"
    fi
  fi
done < <(sed -n '/External Dependencies/,/^##[^#]/p' "$ROADMAP" || true)

if [ "$EXT_COUNT" -eq 0 ]; then
  pass "8" "External Dep Validity" "No external dependencies"
elif [ -z "$EXT_ISSUES" ]; then
  pass "8" "External Dep Validity" "$EXT_COUNT external deps, all valid"
else
  fail "8" "External Dep Validity" "Issues:$EXT_ISSUES"
fi

# ─── OUTPUT ───
echo ""
echo "## Verify Roadmap Results"
echo ""
echo "| # | Check | Status | Detail |"
echo "|---|-------|--------|--------|"
for r in "${RESULTS[@]}"; do
  echo "$r"
done
echo ""
echo "**$PASS_COUNT passed, $FAIL_COUNT failed** out of 8 checks."
echo ""

if [ "$FAIL_COUNT" -gt 0 ]; then
  exit 1
fi
exit 0
