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

/// The spec-less gate (section 0 of the probe fallback) on the taskboard twin, whose phase 02 has no spec.
// Pin: the gate still resolves upstream's compiled spec-section.cjs from install dirs SpecForge never creates, so it
// stops before it reads its toggle. The fix that reads the spec's rows from the graph flips this test.
#[test]
fn the_specless_gate_and_probe_run_in_a_project() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let gate = fence_after(
        &asset("gsd-core/references/specless-probe-fallback.md"),
        "## 0. Gate",
    );
    let (_, err, code) = sh.run_in(
        p.root(),
        &format!(
            "export HOME=\"$PWD/.home\"; unset RUNTIME_DIR; PHASE_ID=v1.0/02; SPEC_FILE=\n{gate}"
        ),
    );
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("spec-section.cjs not found"), "{err}");
}
