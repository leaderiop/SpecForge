//! Ported from gsd-core v1.18.0 tests/perf-317-context-monitor-fs.test.cjs and tests/context-monitor-host-contract.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!

pub(super) const SKIP_FS: &str = "fs: a bridge, sentinel or watermark file's permissions, symlinks and sizes; the state is the kernel's";
pub(super) const SKIP_EVENT: &str = "runtime: the event name and envelope a host wants are the host adapter's; this hook is post_tool only";
pub(super) const SKIP_U53: &str = "diverges: [stopped-at] the stopped-at record is not written by a hook (U53); /gsd-pause-work records it";

pub(super) const MONITOR_SKIPPED: &[(&str, &str)] = &[
    (
        "perf #317: config.json absent (exercises config-missing → defaults path) > respects context_warnings=false",
        "env: the setting is the hook's `when`, so whether it runs is the kernel's; tests/cli/hooks_context.rs runs it on and off",
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > placement: the reset fires even when context warnings are disabled",
        "env: the setting is the hook's `when`, so whether it runs is the kernel's; tests/cli/hooks_context.rs runs it on and off",
    ),
    (
        "#4285 regression: context-monitor thresholds resolve from .planning/config.json > context_warnings:false still wins",
        "env: the setting is the hook's `when`, so whether it runs is the kernel's; tests/cli/hooks_context.rs runs it on and off",
    ),
    ("#1974 context exhaustion auto-record > ", SKIP_U53),
    (
        "#4905: the CRITICAL breadcrumb stamps the local day through the clock seam > ",
        SKIP_U53,
    ),
    (
        "bug #2451, #4985: bridge counts down to compaction and carries the /context tokens > ",
        "diverges: [statusline-metrics] the statusline writes the metrics; its meter arithmetic is checked with the statusline",
    ),
    (
        "bug #925: context monitor echoes the invoking hook event name (superseded for non-injection events by #2289) > ",
        SKIP_EVENT,
    ),
    (
        "bug #925: context monitor falls back to heuristic when hook_event_name absent (non-Gemini fallback now silent per #2289) > ",
        SKIP_EVENT,
    ),
    (
        "bug #925: critical threshold warning also uses correct hookEventName > ",
        SKIP_EVENT,
    ),
    (
        "#2289 context-monitor: non-injection events exit silently > ",
        SKIP_EVENT,
    ),
    (
        "#2289 context-monitor: injection events still warn (unchanged) > AfterTool at 30%",
        SKIP_EVENT,
    ),
    (
        "#2289 context-monitor: injection events still warn (unchanged) > explicit PostToolUse WITH Gemini env",
        SKIP_EVENT,
    ),
    (
        "#2289 context-monitor: side effects still fire on silent events (no output ≠ no side effect) > ",
        SKIP_EVENT,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > AC4",
        SKIP_U53,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > Blocker",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > the truncation fallback",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > O_NOFOLLOW",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 11",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 10",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 3, Minor 6",
        SKIP_EVENT,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 7: a malformed event name",
        SKIP_EVENT,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 7: a planted symlink",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 7: a SYMLINKED",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 7: a non-regular",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 7 control",
        SKIP_FS,
    ),
    (
        "#3709 context-monitor: PreCompact resets the warn sentinel > round 8",
        SKIP_FS,
    ),
    (
        "#3709 round 3: DEBOUNCE_CALLS and STALE_SECONDS at their limits > round 4",
        SKIP_FS,
    ),
    (
        "#3709 round 3: DEBOUNCE_CALLS and STALE_SECONDS at their limits > round 8",
        SKIP_FS,
    ),
];

pub(super) const CONTRACT_SKIPPED: &[(&str, &str)] = &[(
    "#4840: the context monitor is registered only where a metrics writer is installed > ",
    "installer: this port installs the monitor on every host and it is silent without metrics, the install goldens and tests/cli/hooks_context.rs cover that",
)];
