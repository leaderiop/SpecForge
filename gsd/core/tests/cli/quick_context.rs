//! A quick plan's own context, decisions and research (`/gsd-quick --discuss --research`), over
//! `tests/projects/quick-context/`: they name the quick plan in `plan`, as a phase's name it in `phase`.

use crate::harness::{Project, project};

const QUICK: &str = "spec/quick.spec";

/// The `(code, message)` of every `gsd/E206` the project reports.
fn e206(p: &Project) -> Vec<String> {
    p.findings()
        .into_iter()
        .filter(|(code, _)| code == "gsd/E206")
        .map(|(_, message)| message)
        .collect()
}

#[test]
fn a_quick_plan_holds_its_context_and_research() {
    project("quick-context").check_exact(&[]);
}

#[test]
fn a_second_context_of_a_quick_plan_is_e206() {
    let p = project("quick-context");
    let quick = p.read(QUICK);
    p.write(
        QUICK,
        &format!(
            "{quick}\ncontext 251101-abc-CONTEXT-B \"Again\" {{\n  plan     251101-abc\n  boundary \"b\"\n  canonical_refs [ {{ topic \"t\" paths [] }} ]\n}}\n"
        ),
    );
    let found = e206(&p);
    assert!(
        found
            .iter()
            .any(|m| m.contains("context '251101-abc-CONTEXT-B' is a second context")),
        "{found:#?}"
    );
}

#[test]
fn a_context_naming_both_a_phase_and_a_plan_is_e206() {
    let p = project("quick-context");
    p.write(
        "spec/v1.0.spec",
        "milestone v1.0 \"One\" {\n  name \"One\"\n}\n\nphase 01 \"Core\" {\n  milestone v1.0\n  name \"Core\"\n  slug \"core\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    p.edit(
        QUICK,
        "  plan           251101-abc\n  gathered",
        "  phase          v1.0/01\n  plan           251101-abc\n  gathered",
    );
    let found = e206(&p);
    assert!(
        found
            .iter()
            .any(|m| m.contains("context 'v1.0/01/251101-abc-CONTEXT' names both")),
        "{found:#?}"
    );
}

#[test]
fn a_context_of_a_phases_plan_is_e206() {
    let p = project("quick-context");
    p.write(
        "spec/v1.0.spec",
        "milestone v1.0 \"One\" {\n  name \"One\"\n}\n\nphase 01 \"Core\" {\n  milestone v1.0\n  name \"Core\"\n  slug \"core\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}\n\nplan 01-01 \"Core plan\" {\n  phase v1.0/01\n  type execute\n  files_modified [\"src/a.js\"]\n  objective \"o\"\n}\n",
    );
    p.edit(
        QUICK,
        "  plan           251101-abc\n  gathered",
        "  plan           v1.0/01/01-01\n  gathered",
    );
    let found = e206(&p);
    assert!(
        found
            .iter()
            .any(|m| m.contains("context '251101-abc-CONTEXT'") && m.contains("not a quick plan")),
        "{found:#?}"
    );
}

#[test]
fn a_research_of_a_quick_plan_may_have_any_name() {
    let p = project("quick-context");
    p.edit(QUICK, "research 251101-abc-RESEARCH", "research foo");
    p.edit(QUICK, "research   251101-abc-RESEARCH", "research   foo");
    assert_eq!(e206(&p), Vec::<String>::new());
}

/// The "is the quick plan planned" checks the served quick workflows run: every backticked
/// `specforge query "${quick_id}" --kinds ...` command of quick.md and the batch's planner wave.
fn planned_checks(p: &Project) -> Vec<String> {
    let mut found = Vec::new();
    for file in ["quick.md", "quick-batch/steps/planner-wave.md"] {
        let run = p.run(&["gsd", "reference", &format!("gsd-core/workflows/{file}")]);
        assert_eq!(run.code, 0, "{file}: {}", run.all());
        for code in run.stdout.split('`').skip(1).step_by(2) {
            if !code.contains('\n') && code.contains("specforge query \"${quick_id}\" --kinds") {
                found.push(code.to_string());
            }
        }
    }
    found
}

/// Whether `check` succeeds for `quick_id` in the project, run under bash with `specforge` on the `PATH`.
fn holds(p: &Project, check: &str, quick_id: &str) -> bool {
    let bin = std::path::PathBuf::from(std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"));
    let path = format!(
        "{}:{}",
        bin.parent().expect("a directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = std::process::Command::new("bash")
        .args(["-c", check])
        .env("PATH", path)
        .env("quick_id", quick_id)
        .current_dir(p.root())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("bash");
    out.status.success()
}

/// A plan the discussion or research step made first (as the prompt's "The quick plan" says) is not planned; once
/// the planner gives it a task, it is. Each check of the served text agrees.
#[test]
fn a_quick_plan_is_planned_once_it_holds_a_task() {
    let p = project("quick-context");
    let stub = p.run(&[
        "new",
        "plan",
        "251101-stb",
        "Stub",
        "--in",
        QUICK,
        "--type",
        "execute",
        "--files_modified",
        "",
        "--objective",
        "Stub",
    ]);
    assert_eq!(stub.code, 0, "{}", stub.all());
    let checks = planned_checks(&p);
    assert_eq!(checks.len(), 3, "{checks:#?}");
    for check in &checks {
        assert!(
            !holds(&p, check, "251101-stb"),
            "a stub plan counts as planned: {check}"
        );
    }
    let task = p.run(&[
        "new",
        "task",
        "T1",
        "Trim",
        "--in",
        QUICK,
        "--plan",
        "251101-stb",
        "--type",
        "auto",
        "--files",
        "src/tasks.js",
        "--action",
        "Trim the title",
        "--verify_text",
        "The title has no spaces at its ends",
        "--done",
        "The title is trimmed",
    ]);
    assert_eq!(task.code, 0, "{}", task.all());
    for check in &checks {
        assert!(
            holds(&p, check, "251101-stb"),
            "a plan with a task is not planned: {check}"
        );
    }
}
