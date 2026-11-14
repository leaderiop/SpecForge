//! The edge probe of spec-phase's Step 5.5 and plan-phase's spec-less gate, run as the assets write them.

use super::{Shell, asset, fence_after};
use crate::harness::{project, taskboard};

/// Step 5.5's fence on the discuss project, whose phase 02 has a spec: it reads the spec's locked requirements and
/// recorded edges from the graph through `specforge gsd edge-probe`, and prints the engine's report.
#[test]
fn the_spec_phase_edge_probe_runs_in_a_project() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let fence = fence_after(
        &asset("gsd-core/workflows/spec-phase.md"),
        "**Runtime coverage compute",
    );
    let script = format!("unset RUNTIME_DIR; PHASE_ID=v1.0/02\n{fence}");
    let p = project("discuss");
    let run = p.run(&[
        "new",
        "locked_req",
        "LR2",
        "Tasks are saved after every change",
        "--in",
        "spec/discuss.spec",
        "--spec",
        "v1.0/02/02-SPEC",
        "--title",
        "Tasks are saved after every change",
        "--current",
        "Tasks live in memory",
        "--target",
        "Tasks are saved to disk after every change",
        "--acceptance",
        "Each change is on disk before the next one starts",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let report = |out: &str| -> serde_json::Value {
        let open = "### Edge-probe coverage report";
        let body = out
            .split_once(open)
            .map(|(_, rest)| rest)
            .unwrap_or_else(|| panic!("{out}"));
        let body = body
            .split_once('\n')
            .map(|(_, rest)| rest)
            .unwrap_or_default();
        let body = body
            .split("### (end edge-probe coverage report)")
            .next()
            .unwrap_or_default();
        serde_json::from_str(body).unwrap_or_else(|e| panic!("the report is JSON ({e}): {body}"))
    };
    let (out, err, code) = sh.run_in(p.root(), &script);
    assert_eq!(code, 0, "{err}");
    let c = &report(&out)["coverage"];
    assert_eq!(
        (&c["applicable"], &c["unclassified"], &c["unresolved"]),
        (&4.into(), &1.into(), &4.into()),
        "{out}"
    );
    let run = p.run(&[
        "gsd",
        "resolve-edge",
        "v1.0/02/02-SPEC/LR2",
        "adjacency",
        "resolved",
        "--verification",
        "explicit",
        "--resolution",
        "Two changes in one tick are both saved",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let (out, err, code) = sh.run_in(p.root(), &script);
    assert_eq!(code, 0, "{err}");
    let c = &report(&out)["coverage"];
    assert_eq!(
        (&c["resolved"], &c["byVerification"]["explicit"]),
        (&1.into(), &1.into()),
        "{out}"
    );
    // the taskboard's phase 02 has no phase spec: the probe asks for it first
    let (_, err, code) = sh.run_in(taskboard().root(), &script);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("record the phase spec"), "{err}");
}

/// The spec-less gate (section 0 of the probe fallback) and its edge probe (section A): absence is read from the spec's
/// `edge` and `spec_prohibition` rows, and the probe runs on the phase's requirement entities.
#[test]
fn the_specless_gate_and_probe_run_in_a_project() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("gsd-core/references/specless-probe-fallback.md");
    let gate = fence_after(&text, "## 0. Gate");
    let probe = fence_after(&text, "## A. Edge probe");
    let said = "echo \"EDGE_ABSENT=$EDGE_ABSENT PROHIB_ABSENT=$PROHIB_ABSENT SPECLESS_FALLBACK=$SPECLESS_FALLBACK\"";
    // the taskboard's phase 02 has no spec: both sections are absent, and the phase's requirements are probed
    let p = taskboard();
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("unset RUNTIME_DIR; PHASE_ID=v1.0/02; SPEC_FILE=\n{gate}\n{said}\n{probe}\nprintf '%s\\n' \"$COVERAGE\""),
    );
    assert_eq!(code, 0, "{err}");
    let mut lines = out.lines();
    assert_eq!(
        lines.next(),
        Some("EDGE_ABSENT=1 PROHIB_ABSENT=1 SPECLESS_FALLBACK=true"),
        "{out}"
    );
    let report: serde_json::Value = serde_json::from_str(&lines.collect::<Vec<_>>().join("\n"))
        .unwrap_or_else(|e| panic!("{e}: {out}"));
    assert_eq!(
        (
            &report["coverage"]["applicable"],
            &report["coverage"]["unclassified"]
        ),
        (&4.into(), &1.into()),
        "{out}"
    );
    // phase 02.1 has no requirements: a visible skip, not a failure
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("unset RUNTIME_DIR; PHASE_ID=v1.0/02.1; SPEC_FILE=\n{gate}\n{said}"),
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("SPECLESS_FALLBACK=false"), "{out}");
    assert!(err.contains("nothing to probe"), "{err}");
    // the discuss project's spec supplies its edges once a row is recorded, and its prohibitions once one is
    let p = project("discuss");
    let script =
        format!("unset RUNTIME_DIR; PHASE_ID=v1.0/02; SPEC_FILE=v1.0/02/02-SPEC\n{gate}\n{said}");
    let run = p.run(&[
        "gsd",
        "resolve-edge",
        "v1.0/02/02-SPEC/LR1",
        "unclassified",
        "dismissed",
        "--reason",
        "edge-free",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let (out, err, code) = sh.run_in(p.root(), &script);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("EDGE_ABSENT=0 PROHIB_ABSENT=1"), "{out}");
    let run = p.run(&[
        "new",
        "spec_prohibition",
        "SP1",
        "No plaintext",
        "--in",
        "spec/discuss.spec",
        "--spec",
        "v1.0/02/02-SPEC",
        "--requirement",
        "LR1",
        "--statement",
        "MUST NOT store tasks in plaintext",
        "--status",
        "dismissed",
        "--reason",
        "no secrets in a task",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let (out, err, code) = sh.run_in(p.root(), &script);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("PROHIB_ABSENT=0"), "{out}");
}
