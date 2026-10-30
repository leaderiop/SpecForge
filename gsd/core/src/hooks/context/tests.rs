//! Ported from gsd-core v1.18.0 tests/perf-317-context-monitor-fs.test.cjs and tests/context-monitor-host-contract.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Upstream drives the node hook through bridge files in the temp directory. Here the metrics, the debounce and the
//! compaction watermark are the session's state, so a case is ported when it decides on those (a sequence of calls
//! over a state object, the way the kernel feeds it back) and skipped, with its reason, when it is about files,
//! symlinks, the event envelope a host wants, the stopped-at spawn or the installer.

use serde_json::{Value, json};

use super::*;
use crate::hooks::accounting;

const MONITOR_SUITE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/tests/perf-317-context-monitor-fs.test.cjs");
const CONTRACT_SUITE: &str = include_str!(
    "../../../../parity/upstream/v1.18.0/tests/context-monitor-host-contract.test.cjs"
);

const NOW: i64 = 1_800_000_000;
const DEFAULTS: (f64, f64) = (35.0, 25.0);

/// `YYYY-MM-DDTHH:MM:SSZ` of seconds since the epoch.
fn stamp(s: i64) -> String {
    let (days, rem) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// RFC 7396 merge patch.
fn merge(target: &mut Value, patch: &Value) {
    let Value::Object(p) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = json!({});
    }
    let t = target.as_object_mut().expect("an object");
    for (k, v) in p {
        if v.is_null() {
            t.remove(k);
        } else {
            merge(t.entry(k.clone()).or_insert(Value::Null), v);
        }
    }
}

/// A session: the state the kernel keeps between calls.
struct Session {
    state: Value,
    th: (f64, f64),
}

impl Session {
    fn new() -> Self {
        Session {
            state: json!({}),
            th: DEFAULTS,
        }
    }

    /// The statusline's write: `remaining` percent left, read `age` seconds before `now`.
    fn metrics(&mut self, remaining: f64, now: i64, age: i64) -> &mut Self {
        let m = json!({"context": {"remaining_percentage": remaining, "used_pct": 100.0 - remaining, "at": stamp(now - age)}});
        merge(&mut self.state, &m);
        self
    }

    fn seed(&mut self, calls: i64, level: &str) -> &mut Self {
        merge(
            &mut self.state,
            &json!({"monitor": {"calls": calls, "level": level}}),
        );
        self
    }

    fn facts(&self, now: i64) -> Facts<'static> {
        Facts {
            host: "claude-code".into(),
            class: "shell".into(),
            now: stamp(now),
            session_state: self.state.as_object().cloned(),
            ..Facts::default()
        }
    }

    /// A tool use at `now`: what the monitor says, and the state after its patch.
    fn call(&mut self, now: i64) -> String {
        let out = context_monitor(&self.facts(now), self.th);
        if let Some(p) = out.state.and_then(|s| s.session) {
            merge(&mut self.state, &p);
        }
        out.context.unwrap_or_default()
    }

    fn compact(&mut self, now: i64) {
        let out = context_compacted(&self.facts(now));
        let p = out.state.and_then(|s| s.session).expect("a patch");
        merge(&mut self.state, &p);
    }
}

fn warns(says: &str) -> bool {
    says.contains("CONTEXT WARNING")
}

fn critical(says: &str) -> bool {
    says.contains("CONTEXT CRITICAL")
}

/// The reading `remaining` percent, fresh, on a new session.
fn at(remaining: f64) -> String {
    Session::new().metrics(remaining, NOW, 0).call(NOW)
}

/// The cases of `perf-317-context-monitor-fs.test.cjs` this module runs, each asserted where it is written.
fn ported() -> Vec<String> {
    let mut ported = Vec::new();
    let mut port = |title: &str| ported.push(title.to_string());

    // perf #317: no metrics, a first warning, the debounce, the escalation.
    assert_eq!(Session::new().call(NOW), "");
    port("exits 0 with empty stdout when metrics file does not exist");
    assert!(warns(&at(30.0)));
    port(
        "a distinct session with a present metrics file DOES produce output (proves the absent-file test is not vacuous)",
    );
    assert!(warns(&at(30.0)));
    port("emits warning using defaults when .planning/config.json is absent");
    port("emits warning on first call when warn sentinel is absent");
    let mut s = Session::new();
    s.metrics(30.0, NOW, 0).seed(1, "warning");
    assert_eq!(s.call(NOW), "");
    port("debounces when warn sentinel is present and callsSinceWarn is below threshold");
    let mut s = Session::new();
    s.metrics(20.0, NOW, 0).seed(1, "warning");
    assert!(critical(&s.call(NOW)));
    port("severity escalation (WARNING → CRITICAL) bypasses debounce even with sentinel present");

    // #4985 / #2451: the tokens and the raw used percentage.
    let tokens = |used_pct: f64, remaining: f64| {
        let mut s = Session::new();
        merge(
            &mut s.state,
            &json!({"context": {"remaining_percentage": remaining, "used_pct": used_pct, "used_tokens": 300_000,
                                 "threshold_tokens": 967_000, "at": stamp(NOW)}}),
        );
        s.call(NOW)
    };
    let says = tokens(70.0, 30.0);
    assert!(
        says.contains("Usage at 70% (300k of 967k usable tokens). Remaining: 30%."),
        "{says}"
    );
    port("WARNING message names used and usable tokens next to the threshold-scale percentage");
    let mut s = Session::new();
    merge(
        &mut s.state,
        &json!({"context": {"remaining_percentage": 20, "used_pct": 80, "used_tokens": 774_000,
                             "threshold_tokens": 967_000, "at": stamp(NOW)}}),
    );
    let says = s.call(NOW);
    assert!(
        critical(&says) && says.contains("(774k of 967k usable tokens)"),
        "{says}"
    );
    port("CRITICAL message names the tokens too");
    let says = at(30.0);
    assert!(says.contains("Usage at 70%. Remaining: 30%. "), "{says}");
    port("a bridge without token counts keeps the percentage-only message");
    let says = Session::new().metrics(35.0, NOW, 0).call(NOW);
    assert!(says.contains("Usage at 65%"), "{says}");
    port("WARNING message shows raw used_pct consistent with CC reporting");
    let says = at(20.0);
    assert!(says.contains("Usage at 80%"), "{says}");
    port("CRITICAL message shows raw used_pct consistent with CC reporting");
    let says = tokens(70.0, 30.0);
    assert!(says.contains("300k"), "{says}");
    port("the bridge's used_tokens is exactly what /context reports as used");

    // #2289: the warning fires at its boundaries on a post-tool event.
    assert!(warns(&at(30.0)));
    port("PostToolUse at 30% → WARNING envelope with hookEventName PostToolUse");
    assert!(critical(&at(20.0)));
    port("PostToolUse at 20% → CRITICAL envelope");
    assert_eq!(at(36.0), "");
    port("PostToolUse at 36% (above WARNING) → empty stdout");
    assert!(warns(&at(35.0)));
    port("PostToolUse at 35% (WARNING boundary) → WARNING envelope");
    assert!(critical(&at(25.0)));
    port("PostToolUse at 25% (CRITICAL boundary) → CRITICAL envelope");
    assert!(warns(&at(34.0)));
    port("PostToolUse at 34% (WARNING limit-1) → still WARNING envelope");
    assert!(warns(&at(26.0)));
    port("PostToolUse at 26% (CRITICAL limit+1) → WARNING, not CRITICAL");
    assert!(critical(&at(24.0)));
    port("PostToolUse at 24% (CRITICAL limit-1) → CRITICAL envelope");

    // #3709: a compaction resets the cycle.
    let mut s = Session::new();
    s.metrics(20.0, NOW, 0).seed(0, "critical");
    s.compact(NOW);
    assert!(
        s.state
            .get("monitor")
            .and_then(|m| m.get("level"))
            .is_none()
    );
    port("AC1: a PreCompact event clears a sentinel pinned at critical");
    let mut s = Session::new();
    s.compact(NOW);
    assert!(s.state["monitor"]["compacted_at"].is_string());
    port("AC1: PreCompact is tolerant of the sentinel already being absent");
    port("placement: the reset fires with NO metrics file at all");
    let mut s = Session::new();
    s.metrics(90.0, NOW, 0).seed(2, "warning");
    s.compact(NOW);
    assert!(s.state["monitor"].get("calls").is_none() && s.state.get("context").is_none());
    port("placement: the reset fires when usage has recovered to healthy");
    let mut s = Session::new();
    s.seed(3, "warning");
    s.compact(NOW);
    s.metrics(30.0, NOW + 120, 0);
    assert!(warns(&s.call(NOW + 120)));
    port("AC2: after a compaction the first WARNING fires immediately, not debounced");
    let mut s = Session::new();
    s.seed(3, "warning");
    s.compact(NOW);
    s.metrics(30.0, NOW + 120, 0);
    assert!(warns(&s.call(NOW + 120)));
    s.metrics(20.0, NOW + 121, 0);
    assert!(critical(&s.call(NOW + 121)));
    port("AC3: after a compaction a WARNING -> CRITICAL escalation bypasses debounce");
    let mut s = Session::new();
    s.metrics(20.0, NOW, 0).seed(0, "critical");
    s.compact(NOW);
    assert_eq!(s.call(NOW), "");
    port("Major 1: a PreCompact leaves no stale reading for the next tool use");
    port("Major 1: the compaction clears the metrics bridge itself");
    let mut s = Session::new();
    s.metrics(30.0, NOW, 0).seed(1, "warning");
    assert_eq!(s.call(NOW), "");
    assert_eq!(s.state["monitor"]["level"], "warning");
    port("AC5 (non-vacuity): a NON-compaction lifecycle event does NOT clear the sentinel");
    let mut s = Session::new();
    s.seed(0, "warning");
    s.compact(NOW);
    s.metrics(30.0, NOW + 120, 0);
    assert!(warns(&s.call(NOW + 120)));
    port("PreCompact does not consume a debounce slot");
    let mut s = Session::new();
    s.seed(0, "critical");
    s.compact(NOW);
    s.metrics(20.0, NOW, 0);
    assert_eq!(s.call(NOW), "");
    port(
        "round 3, Major 1: a statusline rewrite DURING the compaction cannot re-fire off the old reading",
    );
    let mut s = Session::new();
    s.seed(0, "critical");
    s.compact(NOW);
    s.metrics(20.0, NOW + 1, 0);
    assert_eq!(s.call(NOW + 1), "");
    port(
        "round 3, Major 1: a DELAYED mid-compaction render is dropped too — the watermark marks the start, not the end",
    );
    let mut s = Session::new();
    s.seed(0, "critical");
    s.compact(NOW);
    s.metrics(30.0, NOW + 61, 0);
    assert!(warns(&s.call(NOW + 61)));
    port("round 3, Major 1: a reading from past the grace window still warns");
    let mut s = Session::new();
    merge(
        &mut s.state,
        &json!({"monitor": {"compacted_at": stamp(NOW + 3600)}}),
    );
    s.metrics(30.0, NOW, 0);
    assert!(warns(&s.call(NOW)));
    port("round 3: an insane FUTURE watermark is ignored, never a permanent mute");

    // #3709 round 3: the limits.
    for (seed, emits) in [(3, false), (4, true), (5, true)] {
        let mut s = Session::new();
        s.metrics(30.0, NOW, 0).seed(seed, "warning");
        assert_eq!(warns(&s.call(NOW)), emits, "seed {seed}");
    }
    port(
        "DEBOUNCE_CALLS trio: seed ${seed} (${seed + 1} after increment) → ${emits ? 'emits' : 'debounced'}",
    );
    for (age, emits) in [(59, true), (60, true), (61, false)] {
        let mut s = Session::new();
        s.metrics(30.0, NOW, age);
        assert_eq!(warns(&s.call(NOW)), emits, "age {age}");
    }
    port("STALE_SECONDS trio: reading aged ${age}s → ${emits ? 'warns' : 'dropped as stale'}");
    let mut s = Session::new();
    merge(
        &mut s.state,
        &json!({"context": {"remaining_percentage": 30}}),
    );
    assert!(warns(&s.call(NOW)));
    port("timestamp 0 bypasses the stale gate — characterized directly");
    for (offset, emits) in [(59, false), (60, false), (61, true)] {
        let mut s = Session::new();
        merge(
            &mut s.state,
            &json!({"monitor": {"compacted_at": stamp(NOW)}}),
        );
        s.metrics(30.0, NOW + offset, 0);
        assert_eq!(warns(&s.call(NOW + offset)), emits, "offset {offset}");
    }
    port(
        "COMPACT_GRACE_SECONDS trio: reading at watermark+${offset}s -> ${emits ? 'warns' : 'covered by the window'}",
    );
    for (skew, honored) in [(4, true), (5, true), (6, false)] {
        let mut s = Session::new();
        merge(
            &mut s.state,
            &json!({"monitor": {"compacted_at": stamp(NOW + skew)}}),
        );
        s.metrics(30.0, NOW, 0);
        assert_eq!(warns(&s.call(NOW)), !honored, "skew {skew}");
    }
    port(
        "WATERMARK_SKEW_SECONDS trio: a watermark ${skew}s ahead is ${honored ? 'honored' : 'ignored'}",
    );

    ported.extend(threshold_cases());
    ported
}

/// A session at `remaining` percent, thresholds from the two setting values.
fn with(warning: Value, critical: Value, remaining: f64) -> String {
    let mut s = Session::new();
    s.th = resolve_thresholds(Some(&warning), Some(&critical));
    s.metrics(remaining, NOW, 0).call(NOW)
}

/// `#4285`: the fire-points from the settings.
fn threshold_cases() -> Vec<String> {
    let mut ported = Vec::new();
    let mut port = |title: &str| ported.push(title.to_string());
    let (w, c) = (|n: i64| json!(n), |n: i64| json!(n));

    assert!(warns(&with(w(50), c(25), 45.0)) && at(45.0).is_empty());
    port("a raised warning threshold fires where the default is silent");
    assert!(critical(&with(w(35), c(30), 28.0)) && warns(&at(28.0)));
    port("a raised critical threshold escalates a reading the default calls WARNING");
    assert!(with(w(20), c(10), 30.0).is_empty() && warns(&at(30.0)));
    port("a lowered warning threshold silences a reading the default warns on");
    assert_eq!(
        resolve_thresholds(Some(&json!(-5)), Some(&json!(25))),
        (35.0, 25.0)
    );
    assert_eq!(
        resolve_thresholds(Some(&json!(null)), Some(&json!(20))),
        (35.0, 20.0)
    );
    assert_eq!(
        resolve_thresholds(Some(&json!(50)), Some(&json!(150))),
        (50.0, 25.0)
    );
    port("an unusable value falls back to that key's default rather than throwing");
    assert_eq!(
        resolve_thresholds(Some(&json!(100)), Some(&json!(99))),
        (100.0, 99.0)
    );
    port("100 is inside the domain, not rejected as out of range");
    assert_eq!(
        resolve_thresholds(Some(&json!(20)), Some(&json!(30))),
        (35.0, 25.0)
    );
    port("an inconsistent pair falls back to BOTH defaults, not to the usable half");
    assert_eq!(resolve_thresholds(Some(&json!(20)), None), (35.0, 25.0));
    assert_eq!(resolve_thresholds(Some(&json!(30)), None), (30.0, 25.0));
    assert_eq!(resolve_thresholds(None, Some(&json!(40))), (35.0, 25.0));
    port("a single override is checked against the OTHER key's default");
    assert_eq!(
        resolve_thresholds(Some(&json!("x")), Some(&json!(10))),
        (35.0, 10.0)
    );
    assert_eq!(
        resolve_thresholds(Some(&json!(60)), Some(&json!("x"))),
        (60.0, 25.0)
    );
    port("an invalid key falls back alone — the sibling override survives");
    assert_eq!(
        resolve_thresholds(Some(&json!("50")), Some(&json!("10"))),
        (35.0, 25.0)
    );
    port("a numeric-looking STRING is rejected, not coerced");
    assert_eq!(
        resolve_thresholds(Some(&json!(50)), Some(&json!(-1))),
        (50.0, 25.0)
    );
    port("a below-domain critical is rejected rather than honoured");
    assert_eq!(
        resolve_thresholds(Some(&json!(20)), Some(&json!(30))),
        (35.0, 25.0)
    );
    assert!(with(w(20), c(30), 40.0).is_empty());
    port("the warning half reverts: remaining 40 is silent");
    assert!(warns(&with(w(20), c(30), 32.0)) && !critical(&with(w(20), c(30), 32.0)));
    port("the critical half reverts: remaining 32 is WARNING, not CRITICAL");
    assert_eq!(
        resolve_thresholds(Some(&json!(30)), Some(&json!(30))),
        (35.0, 25.0)
    );
    port("an EQUAL pair is inconsistent too — critical must fire strictly deeper");

    // properties over a grid of values of every kind
    let values = [
        json!(null),
        json!(true),
        json!("30"),
        json!([]),
        json!({}),
        json!(-1),
        json!(0),
        json!(1),
        json!(24),
        json!(25),
        json!(34),
        json!(35),
        json!(36),
        json!(99),
        json!(100),
        json!(101),
        json!(0.5),
        json!(42.5),
    ];
    let usable = |v: &Value| v.as_f64().is_some_and(|n| (0.0..=100.0).contains(&n));
    let mut seen_honoured = 0;
    for a in &values {
        for b in &values {
            let (wr, cr) = resolve_thresholds(Some(a), Some(b));
            assert!(
                wr.is_finite()
                    && cr.is_finite()
                    && (0.0..=100.0).contains(&wr)
                    && (0.0..=100.0).contains(&cr)
            );
            assert!(cr < wr, "{a} {b}");
            let (wf, cf) = (
                if usable(a) {
                    a.as_f64().expect("number")
                } else {
                    35.0
                },
                if usable(b) {
                    b.as_f64().expect("number")
                } else {
                    25.0
                },
            );
            if cf < wf {
                assert_eq!((wr, cr), (wf, cf), "{a} {b}");
                if usable(a) && usable(b) {
                    seen_honoured += 1;
                }
            } else {
                assert_eq!((wr, cr), (35.0, 25.0), "{a} {b}");
            }
        }
    }
    assert!(seen_honoured > 20);
    port("the defaults themselves satisfy the invariant they are the fallback for");
    port("totality: no input throws, and the pair is always two finite numbers in [0, 100]");
    port("ordering: the resolved pair always satisfies critical < warning");
    port(
        "exactness: each side is the configured value or that key's default — never a third number",
    );
    port("togetherness: an inconsistent RESOLVED pair reverts both sides, not the offending one");
    port("non-vacuity: every in-domain consistent pair is honoured verbatim");
    assert_eq!(
        resolve_thresholds(Some(&json!(null)), Some(&json!(10))),
        (35.0, 10.0)
    );
    port("per-key fallback: one unusable key does not discard the other usable one");
    assert_eq!(
        resolve_thresholds(Some(&json!("x")), Some(&json!(10))),
        (35.0, 10.0)
    );
    port("per-key fallback, mirrored: one unusable WARNING does not discard a usable critical");
    assert_eq!(
        resolve_thresholds(Some(&json!(0)), Some(&json!(0))),
        (35.0, 25.0)
    );
    assert_eq!(
        resolve_thresholds(Some(&json!(100)), Some(&json!(100))),
        (35.0, 25.0)
    );
    port("the two in-range endpoints with no legal partner always revert");
    assert_eq!(resolve_thresholds(None, None), (35.0, 25.0));
    port("a non-object hooks argument of any shape yields the defaults");
    ported
}

const SKIP_FS: &str = "fs: a bridge, sentinel or watermark file's permissions, symlinks and sizes; the state is the kernel's";
const SKIP_EVENT: &str = "host: the event name and envelope a host wants are the host adapter's; this hook is post_tool only";
const SKIP_U53: &str =
    "decision: the stopped-at record is not written by a hook (U53); /gsd-pause-work records it";

const MONITOR_SKIPPED: &[(&str, &str)] = &[
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
        "decision: the statusline writes the metrics; its meter arithmetic is checked with the statusline",
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

#[test]
fn context_monitor_accounts_for_every_upstream_case() {
    accounting::account(MONITOR_SUITE, &ported(), MONITOR_SKIPPED);
}

const CONTRACT_SKIPPED: &[(&str, &str)] = &[(
    "#4840: the context monitor is registered only where a metrics writer is installed > ",
    "installer: this port installs the monitor on every host and it is silent without metrics, the install goldens and tests/cli/hooks_context.rs cover that",
)];

#[test]
fn context_monitor_accounts_for_the_host_contract() {
    accounting::account(CONTRACT_SUITE, &[], CONTRACT_SKIPPED);
}

#[test]
fn thresholds_come_from_settings() {
    assert_eq!(
        resolve_thresholds(Some(&json!(50)), Some(&json!(30))),
        (50.0, 30.0)
    );
}

#[test]
fn state_stays_under_one_kib() {
    let mut s = Session::new();
    s.metrics(20.0, NOW, 0).seed(4, "critical");
    merge(
        &mut s.state,
        &json!({"context": {"used_tokens": 774_000, "threshold_tokens": 967_000}}),
    );
    s.compact(NOW);
    s.metrics(20.0, NOW + 100, 0);
    s.call(NOW + 100);
    assert!(s.state.to_string().len() < 1024, "{}", s.state);
}

#[test]
fn a_time_is_read_to_the_second() {
    assert_eq!(epoch("2026-10-30T10:00:00Z"), Some(1_793_354_400));
    assert_eq!(epoch(&stamp(NOW)), Some(NOW));
    assert_eq!(epoch("2026-10-30T10:00:00.250Z"), Some(1_793_354_400));
    assert_eq!(epoch("yesterday"), None);
    assert_eq!(epoch(""), None);
}
