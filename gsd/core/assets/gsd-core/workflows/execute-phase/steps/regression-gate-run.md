<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/execute-phase/steps/regression-gate-run.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
Apply response_language to all user-facing prose — narration between tool calls, status updates, progress notes, and findings included; preserve code, paths, and identifiers.

# Step: regression_gate_run

Run the resolved prior-phase test command one-shot, bounded by a timeout, so a
watch-mode runner (vitest defaults to watch in a TTY; jest `--watch`) cannot
hang this gate forever (#1857). Uses the shared `normalize-test-command` helper
— the same one the post-merge gate uses — so the two gate paths cannot drift.

Reads the prior step's test-file list from the handoff file `regression-files` (#4915); a shell variable
cannot carry it, because each fenced `bash` block runs as its own shell.

```bash
# the commands below are specforge commands: there is no launcher to define
# gsd-handoff:begin regression-files-read
_GSD_HANDOFF="$( { git rev-parse --absolute-git-dir 2>/dev/null || echo "${TMPDIR:-/tmp}/gsd-$(pwd | cksum | cut -d' ' -f1)"; } )/gsd-handoff"; mkdir -p "$_GSD_HANDOFF"
REGRESSION_FILES=""
if [ -s "$_GSD_HANDOFF/regression-files" ]; then
  while IFS= read -r _f; do
    if [ -n "$_f" ]; then REGRESSION_FILES="${REGRESSION_FILES}$(printf '%q ' "$_f")"; fi
  done < "$_GSD_HANDOFF/regression-files"
fi
# gsd-handoff:end regression-files-read
# Resolve test command: project config > Makefile > language sniff
REG_TEST_CMD=""   # workflow.test_command: read it from the settings block; this is its default
if [ -z "$REG_TEST_CMD" ]; then
  if [ -f "Makefile" ] && grep -q "^test:" Makefile; then
    REG_TEST_CMD="make test"
  elif [ -f "Justfile" ] || [ -f "justfile" ]; then
    REG_TEST_CMD="just test"
  elif [ -f "package.json" ]; then
    REG_TEST_CMD="npm test"
  elif [ -f "Cargo.toml" ]; then
    REG_TEST_CMD="cargo test"
  elif [ -f "go.mod" ]; then
    REG_TEST_CMD="go test ./..."
  elif [ -f "requirements.txt" ] || [ -f "pyproject.toml" ]; then
    # gsd-handoff:begin regression-pytest-arm
    if [ -n "$REGRESSION_FILES" ]; then
      REG_TEST_CMD="python -m pytest -q --tb=short -- ${REGRESSION_FILES}"
    else
      echo "⚠ Regression gate: no prior-phase test files were collected — skipping the scoped pytest run instead of falling back to the full suite."
      REG_TEST_CMD="true"
    fi
    # gsd-handoff:end regression-pytest-arm
  else
    REG_TEST_CMD="true"
  fi
fi
# #1857: normalize to a one-shot form (defeat vitest/jest watch mode) and bound
# with a timeout so a watch-mode runner cannot hang the gate indefinitely.
REG_TEST_CMD="$REG_TEST_CMD"   # normalize-test-command is not part of @gsd/core: the command runs as configured
TEST_GATE_TIMEOUT="600"   # workflow.test_gate_timeout: read it from the settings block; this is its default
timeout "$TEST_GATE_TIMEOUT" bash -c "$REG_TEST_CMD" 2>&1
REG_TEST_EXIT=$?
if [ "$REG_TEST_EXIT" -eq 124 ]; then
  echo "✗ REGRESSION GATE ABORTED — test runner did not exit within ${TEST_GATE_TIMEOUT}s, likely stuck in watch/dev mode (e.g. vitest without 'run'). Run tests one-shot (e.g. 'vitest run'), set workflow.test_command, or raise workflow.test_gate_timeout."
fi
```

**On `REG_TEST_EXIT` 124 (`REGRESSION GATE ABORTED`):** HALT — do not proceed to verification. The runner did not exit within the budget (watch/dev mode is the likely cause). Surface the watch-mode cause and the recovery options; never silently continue.
