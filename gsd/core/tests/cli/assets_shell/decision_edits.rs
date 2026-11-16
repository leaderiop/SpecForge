//! The lines that write a decision, run as the assets hold them on a taskboard copy under bash with only `specforge`
//! on the `PATH`: each offers `--supersedes`, `--affects` and `--alternatives`, given only when it holds something.

use super::{Shell, asset, typed};
use crate::harness::taskboard;

/// The PRD express path records the phase's context as typed edits: the context, a decision per requirement (the
/// line repeated with the next id, as a model repeats it) and a deferred idea, on a phase with no context.
#[test]
fn the_prd_express_path_writes_the_phase_context() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/plan-phase/steps/prd-express-path.md");
    let fence = typed::filled(
        &typed::typed_fence(&text, "4. Record the context as typed edits"),
        &[
            ("PHASE_ID", "v1.0/03"),
            ("CONTEXT", "03-CONTEXT"),
            ("TITLE", "Listing context"),
            ("BOUNDARY", "Open tasks are listed and filtered by a word"),
            ("DISCRETION", "The column widths"),
            ("SPECIFICS", "taskboard list prints one task per line"),
            (
                "CANONICAL_REFS",
                r#"[{"topic": "No external specs — requirements fully captured in decisions above", "paths": []}]"#,
            ),
            ("AREA", "Listing"),
            ("DECISION", "The list shows open tasks, oldest first"),
            ("DEFERRED", "Sorting by due date"),
            ("SUPERSEDES", ""),
            ("AFFECTS", "TASK-03"),
            ("ALTERNATIVES", ""),
        ],
    );
    let second = super::line(&fence, "specforge new decision D-01")
        .replace("D-01 \"$DECISION\"", "D-02 \"$DECISION\"");
    let script = format!("{fence}\nDECISION=\"A filter matches a word of the title\"\n{second}");
    let run = typed::apply(&sh, &p, &script);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    for id in ["03-CONTEXT", "D-01", "D-02", "DI1"] {
        let id = format!("v1.0/03/{id}");
        assert!(p.fields_of(&id).is_object(), "{id} is missing");
    }
    assert_eq!(p.fields_of("v1.0/03/D-02")["origin"], "context");
    assert_eq!(
        p.fields_of("v1.0/03/D-01")["affects"],
        serde_json::json!(["TASK-03"])
    );
}

/// The walking skeleton records a project decision that supersedes an earlier one and keeps the option it set aside,
/// whose `why_not` holds an apostrophe; the check gains no finding.
#[test]
fn the_walking_skeleton_fence_records_a_project_decision() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let fence = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/references/skeleton-template.md"),
            "# Walking Skeleton Template",
        ),
        &[
            ("PROJECT", "taskboard"),
            ("DECISION_ID", "D-03"),
            ("CHOICE", "ES modules bundled with esbuild"),
            ("WHY", "The CLI ships as one file"),
            ("SUPERSEDES", "D-01"),
            ("AFFECTS", ""),
            (
                "ALTERNATIVES",
                r#"[{"name": "No build step", "pros": ["node src/cli.js just runs"], "why_not": "The CLI now ships as one file, so it can't stay unbundled"}]"#,
            ),
        ],
    );
    let run = typed::apply(&sh, &p, &fence);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    let query = p.run(&["query", "D-01", "--depth", "1", "--format", "json"]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
    assert!(
        graph["edges"].as_array().expect("edges").iter().any(|e| {
            e["source"] == "D-03" && e["target"] == "D-01" && e["label"] == "supersedes"
        }),
        "{graph}"
    );
    assert_eq!(
        p.fields_of("D-03")["alternatives"][0]["why_not"],
        "The CLI now ships as one file, so it can't stay unbundled"
    );
}
