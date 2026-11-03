//! A truth's `proof` names `@code/core`'s test facts (ADR 0331 D2). `@code/core` is an optional peer of `@gsd/core`:
//! without it a written proof is I004 naming it, and with it a proof no collect has stored facts for is unheld, not
//! E003. Neither way is a W021.

use super::harness::{Project, project};

const ADDS: &str = "js:taskboard/test/tasks.test.js#a task is added with the next id";

/// The `execute` project with TR1 of `01-01` citing `ADDS` in its proof.
fn proved() -> Project {
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        "  verification explicit\n  criterion    1\n",
        &format!("  verification explicit\n  criterion    1\n  proof        [\"{ADDS}\"]\n"),
    );
    p
}

/// Every diagnostic of `specforge check --format json`.
fn diagnostics(p: &Project) -> Vec<serde_json::Value> {
    let run = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    found.as_array().expect("a list").clone()
}

fn coded<'a>(found: &'a [serde_json::Value], code: &str) -> Vec<&'a serde_json::Value> {
    found.iter().filter(|d| d["code"] == code).collect()
}

#[test]
fn a_written_proof_without_code_core_is_i004_naming_it() {
    let found = diagnostics(&proved());
    let i004 = coded(&found, "I004");
    assert_eq!(i004.len(), 1, "{found:#?}");
    assert!(
        i004[0]["suggestion"]
            .as_str()
            .is_some_and(|s| s.contains("specforge add @code/core")),
        "{:#?}",
        i004[0]
    );
    assert!(coded(&found, "E003").is_empty(), "{found:#?}");
}

#[test]
fn a_project_without_code_core_has_no_w021() {
    for p in [project("execute"), proved()] {
        let found = diagnostics(&p);
        assert!(coded(&found, "W021").is_empty(), "{found:#?}");
    }
}

#[test]
fn with_code_core_a_proof_without_facts_is_no_e003() {
    let found = diagnostics(&proved().with_builtins(&["@code/core"]));
    assert!(coded(&found, "E003").is_empty(), "{found:#?}");
    assert!(coded(&found, "I004").is_empty(), "{found:#?}");
}

#[test]
fn with_code_core_a_project_has_no_w021() {
    for p in [project("execute"), proved()] {
        let found = diagnostics(&p.with_builtins(&["@code/core"]));
        assert!(coded(&found, "W021").is_empty(), "{found:#?}");
    }
}
