//! T3: the readers of one JSON answer.

use super::{Shell, after, asset, block, both, line, q, upstream};

/// One rewritten reader: `(asset, start of its line, the variable it reads, the variable it sets, cases)`. A case is
/// `(input, answer, whether upstream's line gives the same answer)`.
type Reader = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str, bool)],
);

/// T3: the one-line readers of a JSON answer. Each case's field path is the one the producing line above it writes:
/// the stubbed `{"concurrency":1}`.
const READERS: &[Reader] = &[
    (
        "gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md",
        "EXEC_OK=$(",
        "EXEC_JSON",
        "EXEC_OK",
        &[
            (
                r#"{"isolation":"orchestrator-worktree","exec":{"command":"codex"}}"#,
                "true",
                true,
            ),
            (
                r#"{"isolation":"orchestrator-worktree","exec":{}}"#,
                "false",
                true,
            ),
            (r#"{"isolation":"none","exec":null}"#, "false", true),
            ("", "false", true),
            ("not json", "false", true),
        ],
    ),
    (
        "gsd-core/references/specless-probe-fallback.md",
        "APPLICABLE=$(",
        "COVERAGE",
        "APPLICABLE",
        &[
            (r#"{"items":[],"coverage":{"applicable":3}}"#, "3", true),
            (r#"{"items":[],"coverage":{"applicable":0}}"#, "0", true),
            ("not json", "0", true),
            // node printed `undefined`
            (r#"{"items":[],"coverage":{}}"#, "0", false),
        ],
    ),
    (
        "gsd-core/workflows/spec-phase.md",
        "APPLICABLE=$(",
        "COVERAGE",
        "APPLICABLE",
        &[
            (r#"{"items":[],"coverage":{"applicable":3}}"#, "3", true),
            ("", "0", true),
        ],
    ),
    (
        "gsd-core/workflows/spec-phase.md",
        "UNCLASSIFIED=$(",
        "COVERAGE",
        "UNCLASSIFIED",
        &[
            (
                r#"{"items":[],"coverage":{"applicable":3,"unclassified":1}}"#,
                "1",
                true,
            ),
            ("not json", "0", true),
        ],
    ),
    (
        "gsd-core/workflows/quick-batch/steps/planner-wave.md",
        "PLAN_CONCURRENCY=$(",
        "QB_PLAN_CONC_JSON",
        "PLAN_CONCURRENCY",
        &[
            (r#"{"concurrency":3}"#, "3", true),
            ("", "1", true),
            ("x", "1", true),
        ],
    ),
    (
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        "RESEARCH_CONCURRENCY=$(",
        "QB_RESEARCH_CONC_JSON",
        "RESEARCH_CONCURRENCY",
        &[(r#"{"concurrency":1}"#, "1", true), ("", "1", true)],
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        "EXEC_CONCURRENCY=$(",
        "QB_EXEC_CONC_JSON",
        "EXEC_CONCURRENCY",
        &[(r#"{"concurrency":2}"#, "2", true), ("x", "1", true)],
    ),
];

#[test]
fn a_field_of_a_json_answer_is_read_with_jq() {
    let Some(sh) = Shell::new() else { return };
    for (path, start, input, output, cases) in READERS {
        let ours = line(&asset(path), start).to_string();
        assert!(!ours.contains("node"), "{path}: {ours}");
        let upstream_text = upstream(path);
        let theirs = line(&upstream_text, start);
        assert!(theirs.contains("node -e"), "{path}: upstream's {theirs}");
        for (json, want, same) in *cases {
            let setup = format!("{input}={}", q(json));
            let show = format!("printf '%s' \"${output}\"");
            both(&sh, &setup, &ours, same.then_some(theirs), &show, want);
        }
    }
}

/// T3: the readers whose line is not `VAR=$(…)`: the harness flag (a continuation line), the verifier's API spot-check (an exit code) and the multi-line ones (remote slug, review verdict).
#[test]
fn the_other_readers_use_jq() {
    let Some(sh) = Shell::new() else { return };
    for path in [
        "gsd-core/references/dispatch-isolation-gate.md",
        "gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md",
    ] {
        let read = |text: &str| {
            format!(
                "HARNESS_FLAG=$(printf '%s' \"$IN\" \\\n{}",
                after(text, "HARNESS_FLAG=$(")
            )
        };
        let (ours, theirs) = (read(&asset(path)), read(&upstream(path)));
        assert!(ours.contains("jq") && theirs.contains("node -e"), "{path}");
        for (json, want) in [
            (
                r#"{"harnessFlag":"isolation=\"worktree\""}"#,
                r#"isolation="worktree""#,
            ),
            ("{}", ""),
            ("x", ""),
        ] {
            let setup = format!("IN={}", q(json));
            both(
                &sh,
                &setup,
                &ours,
                Some(&theirs),
                "printf '%s' \"$HARNESS_FLAG\"",
                want,
            );
        }
    }

    let path = "agents/gsd-verifier.md";
    let start = "curl -s http://localhost:$PORT/api/$ENDPOINT";
    let feed = |l: &str| {
        l.replace(
            "curl -s http://localhost:$PORT/api/$ENDPOINT 2>/dev/null",
            "printf '%s' \"$IN\"",
        )
    };
    let (ours, up) = (feed(line(&asset(path), start)), upstream(path));
    let theirs = feed(line(&up, start));
    for (json, want) in [
        ("[1]", "pass"),
        ("[]", "fail"),
        (r#"{"a":1}"#, "pass"),
        ("{}", "fail"),
        (r#""ab""#, "pass"),
        ("5", "fail"),
        ("null", "fail"),
        ("not json", "fail"),
    ] {
        let setup = format!("IN={}", q(json));
        let show = "[ $? -eq 0 ] && echo pass || echo fail";
        both(
            &sh,
            &setup,
            &format!("{ours} 2>/dev/null"),
            Some(&format!("{theirs} 2>/dev/null")),
            show,
            want,
        );
    }

    let path = "gsd-core/workflows/pr-branch.md";
    let ours = block(&asset(path), "REMOTE_SLUG=$(", "2>/dev/null)");
    let theirs = block(&upstream(path), "REMOTE_SLUG=$(", "\"$RESULT\")");
    for (json, want) in [
        (r#"{"remote_slug":"owner/repo","branch":"x"}"#, "owner/repo"),
        (r#"{"remote_slug":null}"#, ""),
        ("", ""),
        ("x", ""),
    ] {
        let setup = format!("RESULT={}", q(json));
        both(
            &sh,
            &setup,
            &ours,
            Some(&theirs),
            "printf '%s' \"$REMOTE_SLUG\"",
            want,
        );
    }

    let path = "gsd-core/workflows/ship.md";
    let ours = block(&asset(path), "VERDICT=$(", "INVALID_JSON')");
    let theirs = block(&upstream(path), "VERDICT=$(", "\")");
    for (json, want, same) in [
        (r#"{"verdict":"APPROVED","summary":"ok"}"#, "APPROVED", true),
        (r#"{"verdict":"REVISE","issues":[]}"#, "REVISE", true),
        ("not json", "INVALID_JSON", true),
        ("", "INVALID_JSON", true),
        // node printed `undefined` for these
        ("{}", "null", false),
        ("[1]", "INVALID_JSON", false),
    ] {
        let setup = format!("REVIEW_OUTPUT={}", q(json));
        let theirs = same.then_some(theirs.as_str());
        both(&sh, &setup, &ours, theirs, "printf '%s' \"$VERDICT\"", want);
    }
}

/// T3: spec-phase's two guards (and specless-probe-fallback's copies) refuse what node's refused.
#[test]
fn the_edge_probe_guards_use_jq() {
    let Some(sh) = Shell::new() else { return };
    let dir = tempfile::tempdir().expect("a dir");
    let reqs = dir.path().join("reqs.json");
    for path in [
        "gsd-core/workflows/spec-phase.md",
        "gsd-core/references/specless-probe-fallback.md",
    ] {
        let (text, up) = (asset(path), upstream(path));
        let guard = |l: &str| format!("{l} echo bad; else echo ok; fi");
        let start = "if ! ";
        let (ours, theirs) = (guard(line(&text, start)), guard(line(&up, start)));
        assert!(
            ours.contains("REQS_JSON") && theirs.contains("REQS_JSON"),
            "{path}: {ours}"
        );
        for (json, want) in [
            (r#"[{"id":"R1","text":"a user signs in"}]"#, "ok"),
            ("[]", "bad"),
            ("{}", "bad"),
            (
                r#"[{"id":"R1","text":"<replace: requirement text from the SPEC>"}]"#,
                "bad",
            ),
            (r#"[{"id":"R1","text":"  "}]"#, "bad"),
            (r#"[{"id":"R1","text":5}]"#, "bad"),
            ("[null]", "bad"),
            ("not json", "bad"),
        ] {
            std::fs::write(&reqs, json).expect("the requirements");
            let setup = format!("REQS_JSON={}", q(&reqs.display().to_string()));
            both(&sh, &setup, &ours, Some(&theirs), "", want);
        }
        let start = "if ! printf '%s' \"$COVERAGE\"";
        let (ours, theirs) = (guard(line(&text, start)), guard(line(&up, start)));
        for (json, want, same) in [
            (r#"{"items":[],"coverage":{"applicable":1}}"#, "ok", true),
            (r#"{"items":{},"coverage":{}}"#, "bad", true),
            (r#"{"items":[]}"#, "bad", true),
            (r#"{"items":[],"coverage":null}"#, "bad", true),
            ("", "bad", true),
            ("not json", "bad", true),
            // node let an array through as an object
            (r#"{"items":[],"coverage":[]}"#, "bad", false),
        ] {
            let setup = format!("COVERAGE={}", q(json));
            both(
                &sh,
                &setup,
                &ours,
                same.then_some(theirs.as_str()),
                "",
                want,
            );
        }
    }
}
