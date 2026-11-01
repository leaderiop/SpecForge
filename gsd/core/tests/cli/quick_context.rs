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
#[ignore = "G3: plan 02 T2"]
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
#[ignore = "G3: plan 02 T2"]
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
            .any(|m| m.contains("context '251101-abc-CONTEXT' names both")),
        "{found:#?}"
    );
}

#[test]
#[ignore = "G3: plan 02 T2"]
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
#[ignore = "G3: plan 02 T2"]
fn a_research_of_a_quick_plan_may_have_any_name() {
    let p = project("quick-context");
    p.edit(QUICK, "research 251101-abc-RESEARCH", "research foo");
    p.edit(QUICK, "research   251101-abc-RESEARCH", "research   foo");
    assert_eq!(e206(&p), Vec::<String>::new());
}
