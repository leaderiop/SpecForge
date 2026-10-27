//! A writing command on the command line (ADR 0289): its plan is previewed with `--dry-run` and written otherwise,
//! and a plan the host refuses fails the command. The fixture is `@t/core`'s `record`, served in process, answering
//! the plan the test spells.

use super::run_in;
use crate::terminal::{Facts, Terminal};
use serde_json::{Value, json};
use specforge_package::Sha256;
use specforge_project::SharedRuntime;
use specforge_project::testing::{
    CORE, GSD_WRITES_EXTENSIONS, GsdWrites, TestProject, gsd_writes_answer,
};
use specforge_test_macros::test as specforge_test;
use std::sync::Arc;

fn project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file(
            "m.spec",
            "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n",
        )
        .file("03-01.spec", "uat_test U1 \"One\" {\n  phase 03\n}\n")
}

/// `specforge core record <args>`, the command answering a plan that sets `result` of `U1` on `revision`: its exit
/// code, stdout and stderr.
fn record(project: &TestProject, revision: &str, args: &[&str]) -> (i32, String, String) {
    let ops = json!([{"op": "set", "entity": "v1.0/03/U1", "field": "result", "value": "pass",
                      "revision": revision}]);
    let runtime = Arc::new(
        GsdWrites::new()
            .answering(CORE, "cmd__record", gsd_writes_answer(0, ops))
            .runtime(),
    );
    let env = project.environment_in(runtime as SharedRuntime);
    let (terminal, capture) = Terminal::captured("", Facts::PIPED);
    let mut rest = vec!["record".to_string()];
    rest.extend(args.iter().map(ToString::to_string));
    rest.extend(["--path".to_string(), project.root().display().to_string()]);
    let code = run_in(env, &project.project_root(), "core", rest, &[], &terminal);
    (code, capture.stdout(), capture.stderr())
}

fn revision(project: &TestProject) -> String {
    format!(
        "sha256:{}",
        Sha256::of(project.read("03-01.spec").as_bytes())
    )
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a writing command previews its plan with --dry-run and writes it otherwise on the command line"
)]
fn a_writing_command_previews_and_applies_on_the_cli() {
    let p = project();
    let revision = revision(&p);
    let before = p.read("03-01.spec");
    let (code, out, _) = record(&p, &revision, &["--dry-run"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("03-01.spec") && out.contains("+  result \"pass\""),
        "{out}"
    );
    assert_eq!(p.read("03-01.spec"), before);
    let (code, out, _) = record(&p, &revision, &["--format", "json"]);
    assert_eq!(code, 0, "{out}");
    let document: Value = serde_json::from_str(&out).unwrap();
    assert!(
        document["plan"]["affected_files"].is_array() && document.get("output").is_some(),
        "{document}"
    );
    assert!(p.read("03-01.spec").contains("result \"pass\""));
    // The same plan again names a revision the file no longer has.
    let (code, _, err) = record(&p, &revision, &[]);
    assert_eq!(code, 1);
    assert!(err.contains("[stale_revision]"), "{err}");
    let (_, _, err) = record(&p, &revision, &["--format", "json"]);
    assert!(err.contains("\"code\":\"stale_revision\""), "{err}");
}
