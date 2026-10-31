//! Ported from gsd-core v1.19.0 tests/gsd-statusline.test.cjs and tests/gsd-statusline-state.property.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The state formats, the status word and their properties.

use super::*;

use crate::hooks::accounting;

/// The `formatGsdState` and `formatGsdStateCompact` cases.
pub(super) fn state_ported(port: &mut dyn FnMut(&str)) {
    assert_eq!(
        full(&[
            ("milestone", "v1.9"),
            ("milestoneName", "Code Quality"),
            ("status", "executing"),
            ("phaseNum", "1"),
            ("phaseTotal", "5"),
            ("phaseName", "fix-graphiti-deployment")
        ]),
        "v1.9 Code Quality · executing · fix-graphiti-deployment (1/5)"
    );
    port("formatGsdState > formats full state with milestone name, status, and phase name");
    assert_eq!(
        full(&[
            ("milestone", "v1.0"),
            ("milestoneName", "milestone"),
            ("status", "planning")
        ]),
        "v1.0 · planning"
    );
    port("formatGsdState > skips placeholder \"milestone\" value in milestoneName");
    assert_eq!(
        full(&[
            ("milestone", "v2.0"),
            ("status", "executing"),
            ("phaseNum", "3"),
            ("phaseTotal", "7")
        ]),
        "v2.0 · executing · ph 3/7"
    );
    port("formatGsdState > uses short phase form when phase name is missing");
    assert_eq!(
        full(&[("milestone", "v1.0"), ("status", "planning")]),
        "v1.0 · planning"
    );
    port("formatGsdState > omits phase entirely when phaseNum/phaseTotal missing");
    assert_eq!(
        full(&[("milestone", "v1.9"), ("status", "executing")]),
        "v1.9 · executing"
    );
    port("formatGsdState > handles milestone version only (no name)");
    assert_eq!(
        full(&[("milestoneName", "Foundations"), ("status", "planning")]),
        "Foundations · planning"
    );
    port("formatGsdState > handles milestone name only (no version)");
    assert_eq!(
        full(&[("milestone", "v2.0"), ("percent", "100")]),
        "v2.0 [██████████] 100% · milestone complete"
    );
    port("formatGsdState > treats numeric 100 percent as milestone complete (#3153)");
    assert_eq!(full(&[]), "");
    port("formatGsdState > returns empty string for empty state");
    assert_eq!(full(&[("status", "planning")]), "planning");
    port("formatGsdState > returns only available parts when everything else is missing");
    assert_eq!(full(&[("noActiveWorkstream", "")]), "no active workstream");
    port(
        "formatGsdState > renders observable \"no active workstream\" signal for the #2850 sentinel",
    );
    assert_eq!(
        compact(&[("noActiveWorkstream", "")]),
        "no active workstream"
    );
    port(
        "formatGsdStateCompact — #2850 sentinel > renders observable \"no active workstream\" signal",
    );

    // #2833: the lifecycle scenes
    assert_eq!(
        full(&[
            ("milestone", "v2.0"),
            ("status", "executing"),
            ("activePhase", "4.5"),
            ("percent", "59")
        ]),
        "v2.0 [█████░░░░░] 59% · Phase 4.5 executing"
    );
    port(
        "formatGsdState #2833 lifecycle scenes > Scene 1 — active_phase set renders \"Phase X.Y <stage>\"",
    );
    assert_eq!(
        full(&[("milestone", "v2.0"), ("activePhase", "4.5")]),
        "v2.0 · Phase 4.5"
    );
    port(
        "formatGsdState #2833 lifecycle scenes > Scene 1 — active_phase without status renders \"Phase X.Y\"",
    );
    assert_eq!(
        full(&[
            ("milestone", "v2.0"),
            ("nextAction", "execute-phase"),
            ("nextPhases", "4.5"),
            ("percent", "59")
        ]),
        "v2.0 [█████░░░░░] 59% · next execute-phase 4.5"
    );
    port(
        "formatGsdState #2833 lifecycle scenes > Scene 2 — idle + next_action renders \"next <action> <phases>\"",
    );
    assert_eq!(
        full(&[
            ("milestone", "v2.0"),
            ("nextAction", "discuss-phase"),
            ("nextPhases", "4.7,6.5")
        ]),
        "v2.0 · next discuss-phase 4.7/6.5"
    );
    port("formatGsdState #2833 lifecycle scenes > Scene 2 — multiple next_phases joined with /");
    assert_eq!(
        full(&[("milestone", "v2.0"), ("percent", "100")]),
        "v2.0 [██████████] 100% · milestone complete"
    );
    port(
        "formatGsdState #2833 lifecycle scenes > Scene 3 — percent=100 renders \"milestone complete\"",
    );
    assert_eq!(
        full(&[
            ("milestone", "v2.0"),
            ("completedPhases", "17"),
            ("totalPhases", "17")
        ]),
        "v2.0 · milestone complete"
    );
    port(
        "formatGsdState #2833 lifecycle scenes > Scene 3 — completed_phases equals total_phases also triggers complete",
    );
    let out = full(&[
        ("milestone", "v1.15"),
        ("milestoneName", "Design Refresh"),
        ("status", "planning"),
        ("completedPhases", "0"),
        ("totalPhases", "0"),
        ("percent", "0"),
    ]);
    assert!(
        !out.contains("milestone complete") && out.contains("planning"),
        "{out}"
    );
    port(
        "formatGsdState #2833 lifecycle scenes > Scene 3 — 0 of 0 counters does not render milestone complete (#3945)",
    );
    let done = |c: &str, t: &str| {
        full(&[
            ("milestone", "v2.0"),
            ("completedPhases", c),
            ("totalPhases", t),
        ])
        .contains("milestone complete")
    };
    assert!(done("1", "1") && !done("0", "1") && !done("1", "0"));
    port(
        "formatGsdState #2833 lifecycle scenes > Scene 3 — boundary denominators: 1/1 completes, 0/1 and 1/0 do not (#3945)",
    );

    // #2833: backward compatibility
    assert_eq!(
        full(&[
            ("status", "executing"),
            ("milestone", "v1.9"),
            ("milestoneName", "Code Quality"),
            ("phaseNum", "1"),
            ("phaseTotal", "5"),
            ("phaseName", "fix-graphiti-deployment")
        ]),
        "v1.9 Code Quality · executing · fix-graphiti-deployment (1/5)"
    );
    port(
        "formatGsdState #2833 backward compatibility > legacy STATE.md (only status + milestone + phase) renders unchanged",
    );
    assert_eq!(
        full(&[("milestone", "v1.9"), ("status", "executing")]),
        "v1.9 · executing"
    );
    port(
        "formatGsdState #2833 backward compatibility > only status set (no phase, no lifecycle fields) renders just \"<milestone> · <status>\"",
    );
    assert_eq!(full(&[]), "");
    port("formatGsdState #2833 backward compatibility > empty state renders empty string");
    assert_eq!(
        full(&[
            ("milestone", "v1.9"),
            ("milestoneName", "Code Quality"),
            ("status", "executing")
        ]),
        "v1.9 Code Quality · executing"
    );
    port(
        "formatGsdState #2833 backward compatibility > progress.percent is opt-in — absent percent leaves milestone segment unchanged",
    );

    // the bar
    assert!(full(&[("milestone", "v2.0"), ("percent", "0")]).contains("[░░░░░░░░░░] 0%"));
    port("progress bar rendering > 0% renders 10 empty segments");
    assert!(full(&[("milestone", "v2.0"), ("percent", "50")]).contains("[█████░░░░░] 50%"));
    port("progress bar rendering > 50% renders 5 filled + 5 empty");
    assert_eq!(
        full(&[("milestone", "v2.0"), ("percent", "100")]),
        "v2.0 [██████████] 100% · milestone complete"
    );
    port("progress bar rendering > 100% renders 10 filled (and triggers Scene 3)");
    let out = full(&[("milestone", "v2.0"), ("status", "executing")]);
    assert!(!out.contains(['[', '░', '█']), "{out}");
    port("progress bar rendering > percent absent → no bar rendered (opt-in)");
    assert!(full(&[("milestone", "v2.0"), ("percent", "150")]).contains("[██████████] 100%"));
    port("progress bar rendering > percent over 100 clamps to 100");
    assert!(full(&[("milestone", "v2.0"), ("percent", "-10")]).contains("[░░░░░░░░░░] 0%"));
    port("progress bar rendering > percent below 0 clamps to 0");
    assert_eq!(progress_bar(None), "");

    // scene priority
    let out = full(&[
        ("milestone", "v2.0"),
        ("status", "executing"),
        ("activePhase", "4.5"),
        ("nextAction", "execute-phase"),
        ("nextPhases", "4.5"),
    ]);
    assert!(out.contains("Phase 4.5 executing") && !out.contains("next execute-phase"));
    port(
        "formatGsdState #2833 scene priority > active_phase wins over next_action when both populated",
    );
    let out = full(&[
        ("milestone", "v2.0"),
        ("status", "in_progress"),
        ("nextAction", "execute-phase"),
        ("nextPhases", "4.5"),
        ("phaseNum", "1"),
        ("phaseTotal", "5"),
    ]);
    assert!(
        out.contains("next execute-phase 4.5")
            && !out.contains("in_progress")
            && !out.contains("1/5")
    );
    port(
        "formatGsdState #2833 scene priority > next_action wins over Scene 4 fallback when active_phase null",
    );
    let out = full(&[
        ("milestone", "v2.0"),
        ("percent", "100"),
        ("phaseNum", "1"),
        ("phaseTotal", "5"),
    ]);
    assert!(out.contains("milestone complete") && !out.contains("1/5"));
    port("formatGsdState #2833 scene priority > percent=100 wins over Scene 4 even with phase set");

    // the compact format
    assert_eq!(
        compact(&[
            ("milestone", "v1.12"),
            ("phaseNum", "7"),
            ("phaseTotal", "12"),
            ("status", "Executing Phase 7")
        ]),
        "v1.12 · P7/12 · executing"
    );
    assert_eq!(
        compact(&[
            ("milestone", "v1.12"),
            ("phaseNum", "7"),
            ("phaseTotal", "12"),
            ("status", "Executing phase 7 — building the parser")
        ]),
        "v1.12 · P7/12 · Executing"
    );
    port("formatGsdStateCompact > renders version · phase/total · status");
    assert_eq!(
        compact(&[
            ("milestone", "v2.0"),
            ("activePhase", "4.5"),
            ("phaseNum", "4"),
            ("status", "executing")
        ]),
        "v2.0 · P4.5 · executing"
    );
    port("formatGsdStateCompact > prefers lifecycle active_phase over body phase number");
    assert_eq!(
        compact(&[
            ("milestone", "v2.0"),
            ("activePhase", "4.5"),
            ("status", "paused")
        ]),
        "v2.0 · P4.5 · PAUSED"
    );
    port("formatGsdStateCompact > paused state renders uppercase in the compact line");
    assert_eq!(
        compact(&[("milestone", "v2.0"), ("percent", "100")]),
        "v2.0 · complete"
    );
    assert_eq!(
        compact(&[
            ("milestone", "v2.0"),
            ("completedPhases", "5"),
            ("totalPhases", "5")
        ]),
        "v2.0 · complete"
    );
    port("formatGsdStateCompact > milestone completion renders \"complete\"");
    let in_flight = [
        ("milestone", "v2.0"),
        ("activePhase", "4.5"),
        ("percent", "100"),
        ("status", "executing"),
    ];
    assert_eq!(compact(&in_flight), "v2.0 · P4.5 · executing");
    let f = full(&in_flight);
    assert!(!f.contains("complete") || !f.contains("4.5"), "{f}");
    let legacy = [
        ("milestone", "v2.0"),
        ("phaseNum", "5"),
        ("phaseTotal", "5"),
        ("percent", "100"),
        ("status", "verifying"),
    ];
    assert_eq!(compact(&legacy), "v2.0 · P5/5 · complete");
    assert!(full(&legacy).contains("milestone complete"));
    port(
        "formatGsdStateCompact > scene exclusivity: an in-flight phase wins over milestone-complete",
    );
    let cases: [&[(&str, &str)]; 6] = [
        &[("milestone", "v2.0"), ("percent", "100")],
        &legacy,
        &[
            ("milestone", "v2.0"),
            ("completedPhases", "5"),
            ("totalPhases", "5"),
        ],
        &in_flight,
        &[
            ("milestone", "v1.9"),
            ("percent", "40"),
            ("status", "executing"),
            ("phaseNum", "2"),
            ("phaseTotal", "5"),
        ],
        &[
            ("milestone", "v1.15"),
            ("status", "planning"),
            ("completedPhases", "0"),
            ("totalPhases", "0"),
            ("percent", "0"),
        ],
    ];
    for c in cases {
        let done = full(c).contains("milestone complete");
        let cc = compact(c);
        assert_eq!(cc.ends_with(" complete") || cc == "complete", done, "{c:?}");
    }
    port("formatGsdStateCompact > parity: both renderers agree on completion for the same input");
    let out = compact(&[
        ("milestone", "v1.15"),
        ("status", "planning"),
        ("completedPhases", "0"),
        ("totalPhases", "0"),
        ("percent", "0"),
    ]);
    assert!(
        !out.split(' ').any(|w| w == "complete") && out.contains("planning"),
        "{out}"
    );
    port("formatGsdStateCompact > compact — 0 of 0 counters does not render complete (#3945)");
    assert_eq!(
        compact(&[
            ("milestone", "v2.0"),
            ("nextAction", "execute-phase"),
            ("nextPhases", "4.5,4.6")
        ]),
        "v2.0 · next execute-phase 4.5/4.6"
    );
    port("formatGsdStateCompact > idle with queued next action renders \"next <action> <phases>\"");
    assert_eq!(compact(&[]), "");
    port("formatGsdStateCompact > empty state renders empty string");
    let both = [
        ("milestone", "v1.9"),
        ("milestoneName", "Code Quality"),
        ("percent", "40"),
        ("status", "executing"),
        ("phaseNum", "2"),
        ("phaseTotal", "5"),
    ];
    assert!(full(&both).contains("Code Quality"));
    let c = compact(&both);
    assert!(!c.contains("Code Quality") && !c.contains('█'), "{c}");
    port("formatGsdStateCompact > drops the milestone name and progress bar the full format shows");
}

/// The status word cases.
pub(super) fn word_ported(port: &mut dyn FnMut(&str)) {
    assert_eq!(short_status(""), None);
    port("shortGsdStatus > returns null for empty input");
    let w = |s: &str| short_status(s).expect("a word");
    assert_eq!(w("paused"), "PAUSED");
    assert_eq!(w("Paused"), "PAUSED");
    assert_eq!(w("paused — waiting on credentials"), "paused");
    assert_eq!(w("stopped by user"), "stopped");
    port(
        "shortGsdStatus > paused — the canonical stuck state — renders uppercase (#2162 condition)",
    );
    assert_eq!(w("Executing Phase 7"), "executing");
    assert_eq!(w("ready to plan"), "planning");
    assert_eq!(w("Discussing"), "discussing");
    assert_eq!(w("Verifying Phase 2"), "verifying");
    assert_eq!(w("Work complete"), "Work");
    assert_eq!(w("Executing phase 7 of the parser milestone"), "Executing");
    assert_eq!(w("Ready to plan next phase"), "Ready");
    port(
        "shortGsdStatus > collapses vocabulary values to canonical keywords via normalizeStateStatus (#4186 anchored)",
    );
    for c in [
        "discussing",
        "planning",
        "executing",
        "verifying",
        "completed",
        "paused",
    ] {
        assert_eq!(w(c), if c == "paused" { "PAUSED" } else { c });
    }
    port(
        "shortGsdStatus > matches the canonical vocabulary exactly — no drift from normalizeStateStatus",
    );
    assert_eq!(w("reticulating splines"), "reticulating");
    assert_eq!(
        w("supercalifragilisticexpialidocious state"),
        "supercalifragili"
    );
    port("shortGsdStatus > unknown shapes fall back to the first word, capped at 16 chars");
    assert_eq!(w(&"x".repeat(15)), "x".repeat(15));
    assert_eq!(w(&"x".repeat(16)), "x".repeat(16));
    assert_eq!(w(&"x".repeat(17)), "x".repeat(16));
    port("shortGsdStatus > 16-char cap boundary: limit-1 / limit / limit+1");
}

#[test]
fn the_state_properties_account_for_every_case() {
    let mut ported = Vec::new();
    // totality, fixed points, no separators and the first-word fallback, over a grid of strings
    let words = [
        "",
        " ",
        "x",
        "paused",
        "PAUSED",
        "a b",
        "a—b",
        "a–b",
        "a-b",
        "a · b",
        "\t",
        "émoji 💀",
        "executing",
        "Executing Phase 7",
        "ready to plan",
        "verylongwordwithmorethansixteenchars rest",
    ];
    for w in words {
        match short_status(w) {
            None => {}
            Some(out) => {
                assert!(
                    !out.is_empty() && out.chars().count() <= 16,
                    "{w:?} -> {out:?}"
                );
                assert!(
                    !out.contains(|c: char| c.is_whitespace() || matches!(c, '·' | '—' | '–')),
                    "{w:?} -> {out:?}"
                );
            }
        }
    }
    ported.push("shortGsdStatus properties (#2162) > totality: any string input yields null or a short non-empty string".to_string());
    for c in [
        "discussing",
        "planning",
        "executing",
        "verifying",
        "completed",
        "paused",
    ] {
        assert_eq!(
            short_status(c).as_deref(),
            Some(if c == "paused" { "PAUSED" } else { c })
        );
    }
    ported.push("shortGsdStatus properties (#2162) > canonical statuses are fixed points (paused shouts as PAUSED)".to_string());
    ported.push("shortGsdStatus properties (#2162) > output never contains whitespace or separator characters".to_string());
    for (first, second) in [
        ("alpha", "beta"),
        ("zzzzzzzzzzzzzzzzzzzzzzzz", "b"),
        ("Q", "R"),
    ] {
        let out = short_status(&format!("{first} {second}")).expect("a word");
        assert_eq!(out, first.chars().take(16).collect::<String>());
    }
    ported.push("shortGsdStatus properties (#2162) > unrecognized free text falls back to its first word, capped at 16".to_string());
    // the renderers are total and never leave a bad separator, over every combination of present and absent fields
    let bad = |s: &str| {
        s.trim().starts_with('·')
            || s.trim().ends_with('·')
            || s.contains("··")
            || s.contains("· ·")
    };
    let opt = |on: bool, v: &'static str| if on { vec![v] } else { vec![] };
    for mask in 0..(1_u32 << 10) {
        let bit = |n: u32| mask & (1 << n) != 0;
        let mut pairs: Vec<(&str, &str)> = Vec::new();
        for (n, k, v) in [
            (0, "status", "executing"),
            (1, "phaseNum", "3"),
            (2, "phaseTotal", "7"),
            (3, "phaseName", "name"),
            (4, "milestone", "v2.0"),
            (5, "milestoneName", "Name"),
            (6, "percent", "100"),
            (7, "activePhase", "4.5"),
            (8, "nextAction", "execute-phase"),
            (9, "nextPhases", "1,2"),
        ] {
            pairs.extend(opt(bit(n), v).into_iter().map(|v| (k, v)));
        }
        for extra in [
            vec![],
            vec![("completedPhases", "5"), ("totalPhases", "5")],
            vec![("noActiveWorkstream", "")],
        ] {
            let mut p = pairs.clone();
            p.extend(extra);
            let (f, c) = (full(&p), compact(&p));
            assert!(!bad(&f) && !bad(&c), "{p:?}: {f:?} / {c:?}");
        }
    }
    ported.push(
        "state-head freshness properties (#2734) > renderersAreTotalOverArbitraryIr".to_string(),
    );
    // renderersAreTotalOverArbitraryIr is a freshness case in name, a renderer property in fact: ported above
    let mut skipped = STATE_SKIPPED.to_vec();
    skipped.clear();
    skipped.extend([
        ("state-head freshness properties (#2734) > derivation", GIT),
        ("state-head freshness properties (#2734) > fence", GIT),
        ("state-head freshness properties (#2734) > marker", GIT),
        ("state-head freshness properties (#2734) > unknown", GIT),
    ]);
    accounting::account(STATE_SUITE, &ported, &skipped);
}
