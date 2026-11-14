<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/execute-phase/steps/gap-closure-artifacts.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
Apply response_language to all user-facing prose — narration between tool calls, status updates, progress notes, and findings included; preserve code, paths, and identifiers.

<step name="close_parent_artifacts">
**For decimal/polish phases only (X.Y pattern):** Close the feedback loop by resolving parent UAT and debug artifacts.

**Skip if** phase number has no decimal (e.g., `3`, `04`) — only applies to gap-closure phases like `4.1`, `03.1`.

**1. Detect decimal phase and derive parent:**
```bash
# Check if phase_number contains a decimal
if [[ "$PHASE_NUMBER" == *.* ]]; then
  PARENT_PHASE="${PHASE_NUMBER%%.*}"
fi
```

**2. Find parent UAT file:**
```bash
# the commands below are specforge commands: there is no launcher to define
PARENT_INFO=$(specforge query "${PARENT_PHASE}" --depth 0 --format json)
# Extract directory from PARENT_INFO JSON, then find UAT file in that directory
```

**If no parent UAT found:** Skip this step (gap-closure may have been triggered by VERIFICATION.md instead).

**3. The gaps resolve themselves:** a `uat_gap`'s `status` reads resolved once a `gap_closure`
plan naming it completes, and its UAT session's `status` follows from its gaps.

**4. Debug sessions are not entities of `@gsd/core`:** there is none to resolve.

**5. Commit the spec:**
```bash
specforge git commit --add spec -m "docs(phase-${PARENT_PHASE}): resolve UAT gaps and debug sessions after ${PHASE_NUMBER} gap closure" && specforge collect --runner gsd-git --yes
```
</step>
