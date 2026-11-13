//! The discuss family's typed edits, run as the templates hold them on a taskboard copy under bash with only
//! `specforge` on the `PATH`.

use super::{Shell, asset, typed};
use crate::harness::{Project, taskboard};

/// The context template's fence (`gsd-core/templates/context.md`), filled as a model fills it for phase `v1.0/03`.
fn context_fence() -> String {
    typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/templates/context.md"),
            "## The Context's Typed Edits",
        ),
        &[
            ("PHASE_ID", "v1.0/03"),
            ("PADDED_PHASE", "03"),
            ("NAME", "Listing"),
            ("GATHERED", "2026-11-13"),
            (
                "BOUNDARY",
                "Open tasks are listed, oldest first, and filtered by a word",
            ),
            ("SPECIFICS", "Like ls: one task per line"),
            (
                "REFS",
                r#"[{"topic": "No external specs: requirements fully captured in the decisions", "paths": []}]"#,
            ),
            ("DISCRETION", "Column spacing"),
            ("ASSET", "src/board.js: loadBoard reads the tasks to list"),
            ("PATTERN", "One module owns the file"),
            ("INTEGRATION", "The list subcommand of src/cli.js"),
            ("AREA", "Ordering"),
            ("DECISION", "Open tasks are listed oldest first"),
            ("REVERSIBILITY", "reversible"),
            ("RATIONALE", "Only the listing reads the order"),
            ("IDEA", "Sort by due date"),
            ("WHY", "Tasks have no due date yet"),
        ],
    )
}

/// The log template's fence (`workflows/discuss-phase/templates/discussion-log.md`) with `entries` filled.
fn log_fence(entries: &str) -> String {
    typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/discuss-phase/templates/discussion-log.md"),
            "## The Log's Typed Edit",
        ),
        &[
            ("PHASE_ID", "v1.0/03"),
            ("PADDED_PHASE", "03"),
            ("NAME", "Listing"),
            ("ENTRIES", entries),
        ],
    )
}

/// `pick` of `id`, trimmed.
fn picked(p: &Project, id: &str, pick: &str) -> String {
    let run = p.run(&["query", id, "--depth", "0", "--pick", pick]);
    assert_eq!(run.code, 0, "{}", run.all());
    run.stdout.trim().to_string()
}

/// The context fence records the phase context, a decision and a deferred idea: the context is ready for planning and
/// the check reports nothing new.
#[test]
fn the_context_fence_records_the_phase_context() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let run = typed::apply(&sh, &p, &context_fence());
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    assert_eq!(
        picked(&p, "v1.0/03/03-CONTEXT", ".nodes[0].fields.status"),
        "ready_for_planning"
    );
    assert_eq!(
        picked(&p, "v1.0/03/D-01", ".nodes[0].fields.origin"),
        "context"
    );
    assert_eq!(
        picked(&p, "v1.0/03/DI1", ".nodes[0].fields.text"),
        "Sort by due date"
    );
}

/// The log fence records the discussion log, an entry's notes included, and the check reports nothing new.
#[test]
fn the_log_fence_records_the_discussion() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let entries = r#"[{"area": "Ordering", "question": "Which tasks come first?", "options": ["Oldest first — the order they were added", "Newest first — the latest on top"], "chosen": "Oldest first — the order they were added", "mode": "interactive", "notes": "The board is a queue"}, {"area": "Filter", "question": "How is a word matched?", "options": ["Anywhere in the title", "Whole words only"], "chosen": "Anywhere in the title", "mode": "interactive", "notes": ""}]"#;
    let run = typed::apply(&sh, &p, &log_fence(entries));
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    let log = "v1.0/03/03-DISCUSSION-LOG";
    assert_eq!(
        picked(&p, log, ".nodes[0].fields.entries[0].notes"),
        "The board is a queue"
    );
    assert_eq!(
        picked(&p, log, ".nodes[0].fields.entries[1].chosen"),
        "Anywhere in the title"
    );
}
