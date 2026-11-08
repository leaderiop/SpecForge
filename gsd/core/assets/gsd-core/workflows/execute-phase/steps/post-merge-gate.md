<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/execute-phase/steps/post-merge-gate.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Step: post_merge_gate

Post-merge build & test gate. Runs after all worktrees in a wave are merged
(parallel mode), or after the last plan completes (serial mode). Catches
cross-plan integration failures that individual worktree self-checks cannot
detect.

**Step A — Build gate:**

```bash
# the commands below are specforge commands: there is no launcher to define
# Resolve build command: project config > Xcode > Makefile > language sniff: the answer is an argv, never re-parsed
XCODEPROJ=$(specforge files list --depth 2 --kind dir --glob '*.xcodeproj' --pick '.entries[0].path' 2>/dev/null)
XCODE_LIST=""; XCODE_SIMS=""
if [ -n "$XCODEPROJ" ]; then   # machine facts a command cannot read (ADR 0335 D2.2)
  XCODE_LIST=$(xcodebuild -list -json -project "$XCODEPROJ" 2>/dev/null)
  XCODE_SIMS=$(xcrun simctl list devices available -j 2>/dev/null)
fi
BUILD_WORDS=$(specforge gsd gate-command build --xcode-project="$XCODEPROJ" --xcode-list="$XCODE_LIST" --simulators="$XCODE_SIMS" --pick '.argv[]') || exit 1
BUILD_ARGV=(); while IFS= read -r _w; do [ -z "$_w" ] || BUILD_ARGV+=("$_w"); done <<<"$BUILD_WORDS"














































# Run build with 5-minute timeout
BUILD_EXIT=0
if [ ${#BUILD_ARGV[@]} -gt 0 ]; then
  specforge run --deadline 300s -- "${BUILD_ARGV[@]}" 2>&1
  BUILD_EXIT=$?
  if [ "${BUILD_EXIT}" -eq 0 ]; then
    echo "✓ Post-merge build gate passed"
  elif [ "${BUILD_EXIT}" -eq 124 ]; then
    echo "⚠ Post-merge build gate timed out after 5 minutes"
  else
    echo "✗ Post-merge build gate failed (exit code ${BUILD_EXIT})"
    # gsd-handoff:begin wave-failure-record
    _GSD_HANDOFF=$(specforge files temp --dir --name gsd-handoff --pick .path) || exit 1
    printf '.\n' >> "$_GSD_HANDOFF/wave-failures"  # one line per failed gate; a variable would not survive the next block (#4915)
    # gsd-handoff:end wave-failure-record
  fi
fi
```

**If `BUILD_EXIT` is 0 (pass):** `✓ Build gate passed` → proceed to Test gate.

**If `BUILD_EXIT` is 124 (timeout):** Log warning, treat as non-blocking, continue to Test gate.

**If `BUILD_EXIT` is non-zero (build failure):** The block above has already recorded the failure in the cumulative wave-failure handoff file (same semantics as test failures; step 5.8 reads it). Present failure output and offer "Fix now" or "Continue" options (same as step 5.8).

**Step B — Test gate:**

```bash
# Resolve test command: project config > Xcode > Makefile > language sniff: the answer is an argv, never re-parsed
XCODEPROJ=$(specforge files list --depth 2 --kind dir --glob '*.xcodeproj' --pick '.entries[0].path' 2>/dev/null)
XCODE_LIST=""; XCODE_SIMS=""
if [ -n "$XCODEPROJ" ]; then   # machine facts a command cannot read (ADR 0335 D2.2)
  XCODE_LIST=$(xcodebuild -list -json -project "$XCODEPROJ" 2>/dev/null)
  XCODE_SIMS=$(xcrun simctl list devices available -j 2>/dev/null)
fi
TEST_WORDS=$(specforge gsd gate-command test --xcode-project="$XCODEPROJ" --xcode-list="$XCODE_LIST" --simulators="$XCODE_SIMS" --pick '.argv[]') || exit 1
TEST_ARGV=(); while IFS= read -r _w; do [ -z "$_w" ] || TEST_ARGV+=("$_w"); done <<<"$TEST_WORDS"




































# #1857: normalize to a one-shot form (defeat vitest/jest watch mode) via the
# same shared normalize-test-command helper the regression gate uses, then bound
# with the configured timeout so a watch-mode runner cannot hang the gate.
# normalize-test-command is not part of @gsd/core: the command runs as configured
TEST_GATE_TIMEOUT="{{setting:workflow.test_gate_timeout|600}}"   # workflow.test_gate_timeout (the settings)
TEST_EXIT=0
if [ ${#TEST_ARGV[@]} -gt 0 ]; then
  specforge run --deadline "$TEST_GATE_TIMEOUT" -- "${TEST_ARGV[@]}" 2>&1
  TEST_EXIT=$?
fi
if [ "${TEST_EXIT}" -eq 0 ]; then
  echo "✓ Post-merge test gate passed — no cross-plan conflicts"
elif [ "${TEST_EXIT}" -eq 124 ]; then
  echo "⚠ POST-MERGE TEST GATE TIMED OUT after ${TEST_GATE_TIMEOUT}s — the runner did not exit, likely stuck in watch/dev mode (e.g. vitest without 'run'). Verify tests with a one-shot command (e.g. 'vitest run') or raise workflow.test_gate_timeout. For Xcode suites: xcodebuild collects a sysdiagnose into the .xcresult by default (~10 minutes on a passing suite) — add '-collect-test-diagnostics never' to the test command or raise workflow.test_gate_timeout."
else
  echo "✗ Post-merge test gate failed (exit code ${TEST_EXIT})"
  # gsd-handoff:begin wave-failure-record
  _GSD_HANDOFF=$(specforge files temp --dir --name gsd-handoff --pick .path) || exit 1
  printf '.\n' >> "$_GSD_HANDOFF/wave-failures"  # one line per failed gate; a variable would not survive the next block (#4915)
  # gsd-handoff:end wave-failure-record
fi
```

**If `TEST_EXIT` is 0 (pass):** `✓ Post-merge test gate: {N} tests passed — no cross-plan conflicts` → continue to orchestrator tracking update.

**If `TEST_EXIT` is 124 (timeout):** The runner did not exit within the budget — surface the printed message clearly (watch/dev mode is the likely cause; #1857). Treated as non-blocking (a genuinely long suite may just need a larger `workflow.test_gate_timeout`), but it is NEVER silently ignored — the watch-mode cause is named so the user can fix it (one-shot command / `workflow.test_command` / larger timeout).

**If `TEST_EXIT` is non-zero (test failure):** The block above has recorded the failure in the cumulative wave-failure handoff file, which tracks
failures across waves (a shell counter cannot: every fenced block is its own shell, #4915). Subsequent waves should report:
`⚠ Note: ${WAVE_FAILURE_COUNT} prior wave(s) had test failures` (`WAVE_FAILURE_COUNT` is read from the handoff file, as in step 5.8 of `execute-phase.md`)
