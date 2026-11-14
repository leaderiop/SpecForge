<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/plan-phase/steps/research-only-modifiers.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
### 5.0. Research-Only Modifiers (`--view`, `--research`)

**Skip if:** `RESEARCH_ONLY` is `false`.

Three branches in research-only mode (`--research-phase <N>`):

1. **`--view`**: print the phase's research with its findings, pitfalls and other items (`specforge query "$PHASE_ID" --depth 2 --kinds research,finding,stack_choice,pattern,pitfall,dont_hand_roll,open_question --format context`), spawn nothing and exit. With no research recorded for the phase, stop on the error "--view needs a recorded research; run without --view to spawn the researcher."
2. **`--research`** (force-refresh): re-spawn researcher unconditionally — fall through to "Spawn gsd-phase-researcher" below.
3. **Neither flag AND `has_research=true`:** auto-use the existing research and exit cleanly — do not prompt, do not re-spawn. Print "Phase ${PHASE} has its research already; using it. --research re-runs the researcher, --view prints it." and stop. The explicit-flag escape hatches cover any deviation; this matches §5.1's promptless auto-use of existing research, removing the §5.0/§5.1 inconsistency (#159).

```bash
if [[ "$VIEW_ONLY" == "true" ]]; then
  RESEARCH_ID=$(specforge query "$PHASE_ID" --depth 1 --kinds research --pick '.nodes[kind=research].id'); [ -n "$RESEARCH_ID" ] || { echo "Error: --view requires an existing research (Phase ${PHASE}). Drop --view to spawn the researcher." >&2; exit 1; }
  specforge query "$RESEARCH_ID" --format context; exit 0
fi
```
