<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/plan-phase/steps/stall-detection-helpers.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Bounded Stall-Detection Helpers (#2650)

`planner.stall_detection_enabled` controls the wait policy for the five scoped
planner/plan-checker spawns. It defaults to `true`. In that default-on mode each
spawn dispatches with `run_in_background=true`, then the orchestrator runs the
**watch cycle** below, one Bash call at a time, until the spawn reaches
`marker_received` or `stalled`. This mirrors the already-shipped `executor.stall_*`
pattern (`execute-phase.md`, bug #3212, commit `e7942c21b`) but — unlike
that prose-only surveillance, which cannot run during a *blocking* `Agent()`
call — each watch cycle is a real, bounded `specforge wait` issued as its own
tool call, so it returns control to the orchestrator on its own schedule
regardless of whether the backgrounded agent's own completion notification
ever arrives.

When the key is explicitly the JSON boolean `false`, each scoped call instead
omits `run_in_background`, waits through the runtime-native ordinary Agent()
completion mechanism, and consumes that real returned result. It never runs
the watch cycle, so there are no periodic waits. This is still a wait,
not fire-and-forget. It deliberately gives up #2650's bounded automatic
recovery: if the runtime loses the completion handoff, the user may need to
interrupt and use the existing filesystem fallback. The completion markers and
empty/truncated/unrecognized-return fallback remain unchanged.

**One watch cycle, one Bash call:** each host Bash call is a new shell, so no
function or variable outlives it. A cycle is one line of the blocks below, the
one for the spawn's mode, its settings filled in: it waits one `planner.stall_detect_interval_minutes`
interval, then looks once for a file written during it where that planner writes: `**/*.spec`
from the project root (GSD on SpecForge writes plans and their tasks as entities in `.spec` files),
or the outline planner's outline, a temp file. It prints `changed` (exit 0: the spawn is
**active**) or `deadline` (exit 124: the interval was **quiet**). Any other exit means the cycle could not watch (a refusal, a bad setting, no `specforge`): it prints `unwatched: exit <n>`, the refusal on stderr; stop and report it, never classify it `active` or `stalled`.

**After each cycle, read `{outputFile}` with the Read tool (load-bearing, not
optional):** `{outputFile}` is the REAL path the immediately preceding
`run_in_background=true` Agent() call returned in its `async_launched` result,
read exactly as `docs-update.md:471` reads one ("Read tool: file_path:
`{outputFile from README agent result}`"). It lies in the host's temp
directory, outside the project, so no `specforge` verb reads it. Then classify the spawn:
- a line of `{outputFile}` holding one of the spawn's completion markers: `marker_received`, and the wait ends;
- else a quiet cycle once the cycles run so far add up to `planner.stall_threshold_minutes`: `stalled`, and the wait ends;
- else (`active`, or quiet before the threshold: waiting) run the next cycle.
Every cycle that did not end on a marker lasted one full interval, so the count of cycles times the
interval is the time since dispatch.

**Plan-checker's signal is its marker, not activity:** a plan-checker that
PASSES writes no `.spec` file — no activity, ever, on a clean run. Without
`{outputFile}` read after each cycle, a healthy plan-checker that returns
`## VERIFICATION PASSED` in two minutes would still be declared `stalled`
once `planner.stall_threshold_minutes` elapses — reporting a succeeded agent
as hung, which is worse than the original unbounded wait. The marker is the
ONLY working completion signal for that spawn; activity is secondary there.

**Single-cycle by design, not one long-lived loop:** a watch cycle waits for
exactly one `planner.stall_detect_interval_minutes` and returns — it does NOT
wait for the full `planner.stall_threshold_minutes`. A single Bash
tool call blocking for `threshold + interval` minutes (up to 15 min at
defaults) risks the *host tool's own* timeout killing the call before it ever
prints a result — silently defeating the fix it exists to ship. Looping at
the orchestrator-prose level instead means every cycle is a short (default 5
min), real, bounded call that reliably hands control back — the outer
threshold is enforced by the cycles the orchestrator counts across calls,
not by one call's own duration.

**Never a wake-up call between cycles (#4079):** while the orchestrator waits
between watch cycles, it must NOT call `ScheduleWakeup` (or any
host wake/sleep-scheduling tool, e.g. the `/loop` pacing surface) to
literalize "I'll wait". The watch cycle's Bash call IS the wait mechanism;
wake-up scheduling is never part of it, and a partial-args `ScheduleWakeup`
call surfaces the host's red validation error (`prompt` is required when
`stop` is not true). Just issue the next watch cycle (or let the blocking
Agent() return) — nothing else.

**Disclosed tradeoff:** the first cycle always waits a full
`planner.stall_detect_interval_minutes` before its first check, so a planner that
completes in seconds is not observed by this path until that interval
elapses (default 5 min) — slower than a plain blocking call's near-instant
return on success. This is deliberate: it trades a bounded, at-most-one-
interval delay on the (common) success path for eliminating the unbounded,
possibly-indefinite hang on the (rare, previously unrecoverable) stall path
this issue is about. `planner.stall_detect_interval_minutes` is the knob for
projects that want a tighter success-path latency at the cost of more watch cycles.
This block is independent of, and never gated behind, the `query
teams-status` / `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` guard used for the
researcher spawn — the stall path applies on every runtime, teams-active or
not (AC2). The toggle is resolved through the canonical `config-get` seam, so
root/project/workstream selection stays with the Config Loader rather than a
workflow-local JSON parser. Only the exact boolean result `false` disables;
missing, malformed, string, numeric, or otherwise unrecognized values fail safe
to `true`.

```bash
# the commands below are specforge commands: there is no launcher to define
PLANNER_STALL_DETECTION_ENABLED="{{setting:planner.stall_detection_enabled|true}}"   # planner.stall_detection_enabled (the settings)
# Defense in depth for hand-edited config and old runtimes: only the exact
# canonical false token may disable the default-on recovery policy.
[ "$PLANNER_STALL_DETECTION_ENABLED" = "false" ] || PLANNER_STALL_DETECTION_ENABLED=true
PLANNER_STALL_INTERVAL_MINUTES="{{setting:planner.stall_detect_interval_minutes|5}}"   # planner.stall_detect_interval_minutes (the settings)
PLANNER_STALL_THRESHOLD_MINUTES="{{setting:planner.stall_threshold_minutes|10}}"   # planner.stall_threshold_minutes (the settings)
```

The watch cycle, one Bash call each time (the settings are filled in, since no shell variable outlives a call), for a planner that writes entities (step 8, a chunked single-plan planner, a revision) and for a plan-checker:

```bash
CYCLE_START=$(specforge wait --for "{{setting:planner.stall_detect_interval_minutes|5}}m" --pick .started_at) && specforge wait --for 0 --changed '**/*.spec' --since "$CYCLE_START" --pick .reason || { rc=$?; [ "$rc" = 124 ] || echo "unwatched: exit $rc"; exit "$rc"; }
```

The outline watch cycle, for the chunked outline planner (§8.5.1), which writes only its outline, a temp file `specforge files temp --name` made:

```bash
CYCLE_START=$(specforge wait --for "{{setting:planner.stall_detect_interval_minutes|5}}m" --pick .started_at) && specforge wait --for 0 --changed '.specforge/tmp/gsd-outline-*.md' --since "$CYCLE_START" --pick .reason || { rc=$?; [ "$rc" = 124 ] || echo "unwatched: exit $rc"; exit "$rc"; }
```
