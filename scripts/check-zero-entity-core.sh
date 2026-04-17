#!/usr/bin/env bash
set -euo pipefail

# Verify that core crate source directories contain no hardcoded entity kind names.
# Extension code (builtins/) is excluded — it SHOULD reference domain vocabulary.
# Test files, doc comments, and lines marked exempt are excluded.
#
# "type", "property", "channel", "event" are excluded because they collide
# with JSON Schema keywords, LSP protocol terms, and MCP event contexts.

ENTITY_PATTERN='"behavior"|"feature"|"invariant"|"port"|"decision"|"constraint"|"failure_mode"|"journey"|"deliverable"|"milestone"|"module"|"term"|"persona"|"release"|"condition"|"axiom"|"refinement"|"process"'

CORE_DIRS=(
  crates/specforge-parser/src
  crates/specforge-resolver/src
  crates/specforge-validator/src
  crates/specforge-emitter/src
  crates/specforge-graph/src
  crates/specforge-lsp/src
  crates/specforge-mcp/src
)

EXIT_CODE=0

for dir in "${CORE_DIRS[@]}"; do
  if [ ! -d "$dir" ]; then
    continue
  fi

  hits=$(grep -rn --include='*.rs' -E "$ENTITY_PATTERN" "$dir" \
    | grep -v '/builtins/' \
    | grep -v '// zero-entity-core: exempt' \
    | grep -v '/tests/' \
    | grep -v '#\[test\]' \
    | grep -v '^\s*//' \
    | grep -v '///' \
    || true)

  if [ -n "$hits" ]; then
    echo "ERROR: Hardcoded entity kind names found in $dir:"
    echo "$hits"
    echo
    EXIT_CODE=1
  fi
done

if [ "$EXIT_CODE" -eq 0 ]; then
  echo "OK: No hardcoded entity kind names in core crate sources."
fi

exit "$EXIT_CODE"
