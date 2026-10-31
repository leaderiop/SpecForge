<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/verify-work/steps/mvp-uat-framing.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**MVP-mode UAT framing.** When `MVP_MODE=true`, follow the rules in `@~/.claude/gsd-core/references/verify-mvp-mode.md`. Briefly:

1. Generate the UAT script in three ordered sections: (a) user-flow walk-through derived from the phase's user-story goal, (b) technical checks (deferred — only run after user flow passes), (c) coverage check (goal-backward, narrowed to the user story's outcome clause).
2. **User-flow steps run first.** Each step is one user action: open, fill, click, type, observe. No HTTP verbs, no JSON shapes, no error codes in user-flow steps.
3. **Technical checks are deferred.** They run AFTER the user flow passes — same checks as non-MVP mode (endpoint schemas, error states, edge cases), just reordered.
4. **If user-flow step N fails, do not advance.** The verdict is FAIL; technical checks do not run. The user can re-run after fixing the underlying flow.

**User-story format guard.** When `MVP_MODE=true`, also verify the phase's goal is in User Story format via the centralized validator:

```bash
IFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true
$ARGUMENTS
GSD_ARGS_END
GSD_ARGS=${GSD_ARGS%$'\n'}
# the commands below are specforge commands: there is no launcher to define
# #4772: this file is Read, not textually substituted - write the parent command's original argument string in place of $ARGUMENTS so --ws reaches the query.
GSD_WS=$(echo " $GSD_ARGS" | sed -nE 's/.* --ws +([A-Za-z0-9][A-Za-z0-9._-]*).*/--ws \1/p' | head -n 1)
PHASE_GOAL=$(specforge query "$PHASE_ID" --depth 0 --format json | jq -r '.nodes[0].fields.goal')
USER_STORY_VALID=$(printf '%s' "$PHASE_GOAL" | grep -qiE '^as an? .+, i want .+ so that .+' && echo true || echo false)
if [ "$USER_STORY_VALID" != "true" ]; then
  echo "Phase ${phase_number} has '**Mode:** mvp' in ROADMAP.md but the **Goal:** is not in user-story format."
  echo "Run /gsd:mvp-phase ${phase_number} to set a user-story goal before verifying."
  exit 1
fi
```

The verb owns the canonical regex `/^As a .+, I want to .+, so that .+\.$/` and returns slot extractions plus per-error guidance when invalid. Halt UAT generation on failure — never attempt to derive user-flow steps from a non-User-Story goal (low-quality UAT).
