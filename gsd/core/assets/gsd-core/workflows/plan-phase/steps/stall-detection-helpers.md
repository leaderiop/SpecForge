<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/plan-phase/steps/stall-detection-helpers.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Bounded Stall-Detection Helpers (#2650)

`planner.stall_detection_enabled` controls the wait policy for the five scoped
planner/plan-checker spawns. It defaults to `true`. In that default-on mode each
spawn dispatches with `run_in_background=true`, records `TS=$(date +%s)`, and
then repeatedly calls `gsd_stall_watch` until it returns something other than
`waiting`/`active`. This mirrors the already-shipped `executor.stall_*`
pattern (`execute-phase.md`, bug #3212, commit `e7942c21b`) but — unlike
that prose-only surveillance, which cannot run during a *blocking* `Agent()`
call — each `gsd_stall_watch` call is a real, bounded bash subprocess wait
issued as its own tool call, so it returns control to the orchestrator on
its own schedule regardless of whether the backgrounded agent's own
completion notification ever arrives.

When the key is explicitly the JSON boolean `false`, each scoped call instead
omits `run_in_background`, waits through the runtime-native ordinary Agent()
completion mechanism, and consumes that real returned result. It never calls
`gsd_stall_watch`, so there are no periodic shell sleeps. This is still a wait,
not fire-and-forget. It deliberately gives up #2650's bounded automatic
recovery: if the runtime loses the completion handoff, the user may need to
interrupt and use the existing filesystem fallback. The completion markers and
empty/truncated/unrecognized-return fallback remain unchanged.

**Binding `{outputFile}` (load-bearing, not optional):** every `gsd_stall_watch`
call below takes `{outputFile}` as its second argument — a literal token the
orchestrator must substitute with the REAL path from the immediately preceding
`run_in_background=true` Agent() call's returned `async_launched` result,
exactly as `docs-update.md:471` already does ("Read tool: file_path: `{outputFile
from README agent result}`"). This is NOT a bash variable the snippet below
assigns — there is nothing upstream that assigns one, so a bash variable
reference here would silently stay empty forever. With `{outputFile}` correctly
substituted, `[ -f "$output_file" ]` can find the real file and the
`marker_received` path is reachable; left as a literal (or as an unbound bash
variable), `marker_found` can never become `true` and every spawn silently
falls back to the mtime-only path — for the plan-checker spawn specifically,
that fallback is broken (see next paragraph), so binding this correctly there
is not a nice-to-have.

**Plan-checker's artifact glob needs the marker, not just mtimes:** the
plan-checker spawn watches `*-PLAN.md` for freshness, but a checker that
PASSES touches none of those files — no fresh mtime, ever, on a clean run.
Without `{outputFile}` correctly bound to the real completion output, a
healthy plan-checker that returns `## VERIFICATION PASSED` in two minutes
would still be declared `stalled` once `planner.stall_threshold_minutes`
elapses — reporting a succeeded agent as hung, which is worse than the
original unbounded wait. The marker path (via `{outputFile}`) is the ONLY
working completion signal for that spawn; the artifact glob is secondary
there.

**Single-cycle by design, not one long-lived loop:** `gsd_stall_watch` sleeps
for exactly one `PLANNER_STALL_INTERVAL_MINUTES` and returns — it does NOT
loop internally for the full `PLANNER_STALL_THRESHOLD_MINUTES`. A single Bash
tool call blocking for `threshold + interval` minutes (up to 15 min at
defaults) risks the *host tool's own* timeout killing the call before it ever
prints a result — silently defeating the fix it exists to ship. Looping at
the orchestrator-prose level instead means every cycle is a short (default 5
min), real, bounded call that reliably hands control back — the outer
threshold is enforced by `dispatch_ts` accumulating across calls, not by one
call's own duration.

**Never a wake-up call between cycles (#4079):** while the orchestrator waits
between `gsd_stall_watch` cycles, it must NOT call `ScheduleWakeup` (or any
host wake/sleep-scheduling tool, e.g. the `/loop` pacing surface) to
literalize "I'll wait". The `gsd_stall_watch` bash call IS the wait mechanism;
wake-up scheduling is never part of it, and a partial-args `ScheduleWakeup`
call surfaces the host's red validation error (`prompt` is required when
`stop` is not true). Just issue the next watch call (or let the blocking
Agent() return) — nothing else.

**Disclosed tradeoff:** the first cycle always sleeps a full
`PLANNER_STALL_INTERVAL_MINUTES` before its first check, so a planner that
completes in seconds is not observed by this path until that interval
elapses (default 5 min) — slower than a plain blocking call's near-instant
return on success. This is deliberate: it trades a bounded, at-most-one-
interval delay on the (common) success path for eliminating the unbounded,
possibly-indefinite hang on the (rare, previously unrecoverable) stall path
this issue is about. `PLANNER_STALL_INTERVAL_MINUTES` is the knob for
projects that want a tighter success-path latency at the cost of more
config-get calls.

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
# Both values are config-controlled (the settings (`specforge settings`), editable by any repo
# contributor) and both flow into `$(( ))` arithmetic below. A non-numeric
# value there is NOT a code-execution risk (empirically verified: bash's
# arithmetic evaluator hard-errors on a `$(cmd)`-shaped operand instead of
# invoking it — "syntax error: operand expected", command never runs) but IS
# a reliability risk this fix cannot afford: a malformed config value would
# abort the stall-watcher itself with a bash syntax error, silently defeating
# the exact hang-recovery this issue is about. Reject anything that is not a
# bare non-negative integer before it is ever used, so a bad config value
# degrades to the safe default instead of crashing the watcher.
[[ "$PLANNER_STALL_INTERVAL_MINUTES" =~ ^[0-9]+$ ]] || PLANNER_STALL_INTERVAL_MINUTES=5
[[ "$PLANNER_STALL_THRESHOLD_MINUTES" =~ ^[0-9]+$ ]] || PLANNER_STALL_THRESHOLD_MINUTES=10

# gsd_stall_should_recover — pure decision function, no IO, no sleeping. Given how
# long the orchestrator has been waiting plus two liveness signals (a completion
# marker found in the agent's output file, and fresh on-disk artifact activity),
# decides whether to keep waiting, treat the wait as satisfied, or auto-surface the
# existing accept/retry/stop recovery menu (9a/11a). Never kills or retries anything
# itself — it only classifies. Re-validates both numeric args as bare non-negative
# integers (defense in depth — safe to call with any input, not just the resolved
# config globals above) before either ever reaches arithmetic expansion.
gsd_stall_should_recover() {
  local elapsed_seconds="$1" threshold_minutes="$2" marker_found="$3" artifact_fresh="$4"
  [[ "$elapsed_seconds" =~ ^[0-9]+$ ]] || elapsed_seconds=0
  [[ "$threshold_minutes" =~ ^[0-9]+$ ]] || threshold_minutes=10
  local threshold_seconds=$(( threshold_minutes * 60 ))
  if [ "$marker_found" = "true" ]; then
    echo "marker_received"; return 0
  fi
  if [ "$artifact_fresh" = "true" ]; then
    echo "active"; return 0
  fi
  if [ "$elapsed_seconds" -ge "$threshold_seconds" ]; then
    echo "stalled"; return 0
  fi
  echo "waiting"; return 0
}

# gsd_stall_watch — ONE bounded, real (non-LLM-side) sleep-and-check cycle, not
# a long-lived loop (see "Single-cycle by design" above — a single Bash tool
# call spanning the full threshold risks the host tool's own timeout killing
# it first). Sleeps exactly one PLANNER_STALL_INTERVAL_MINUTES, then checks for
# a completion marker in $2 (the outputFile returned by the run_in_background
# Agent() call) or fresh mtime activity under $3 (an artifact glob), against
# elapsed time since $1 (an epoch-seconds dispatch_ts the CALLER records once,
# before the first call, and passes unchanged on every repeat). Remaining args
# are completion markers. Prints exactly one of: marker_received | active |
# waiting | stalled. The caller repeats the call while the result is
# waiting/active; any other result ends the wait.
gsd_stall_watch() {
  local dispatch_ts="$1" output_file="$2" artifact_glob="$3"; shift 3
  local markers=("$@")
  [[ "$dispatch_ts" =~ ^[0-9]+$ ]] || dispatch_ts=$(date +%s)
  sleep "$(( PLANNER_STALL_INTERVAL_MINUTES * 60 ))"
  local now elapsed marker_found artifact_fresh
  now=$(date +%s)
  elapsed=$(( now - dispatch_ts ))
  marker_found="false"
  if [ -f "$output_file" ]; then
    for m in "${markers[@]}"; do
      if grep -qF "$m" "$output_file" 2>/dev/null; then marker_found="true"; break; fi
    done
  fi
  # -mmin -N ("modified less than N minutes ago"), not -newermt "@<epoch>":
  # -newermt's "@<epoch>" shorthand is a GNU-date convenience the shipped
  # BSD find(1) on macOS does NOT understand ("Can't parse date/time:
  # @<epoch>", verified live) — with the 2>/dev/null below that failed
  # silently and permanently degraded artifact_fresh to false on every
  # macOS run. -mmin -N needs no epoch/date-string conversion at all and is
  # supported identically by GNU find (Linux, Git-for-Windows' bundled
  # findutils) and BSD find (macOS). $artifact_glob stays intentionally
  # unquoted — the shell, not find, expands it into the matching file list.
  artifact_fresh="false"
  if [ -n "$(find $artifact_glob -mmin "-${PLANNER_STALL_INTERVAL_MINUTES}" 2>/dev/null)" ]; then
    artifact_fresh="true"
  fi
  gsd_stall_should_recover "$elapsed" "$PLANNER_STALL_THRESHOLD_MINUTES" "$marker_found" "$artifact_fresh"
}
```
