//! A `use` binding's imported name as the LSP serves it (ADR 0233): a reference, a prepareRename range and a
//! rename edit; the alias names nothing.

use specforge_lsp::answers;
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{Location, Position, PrepareRenameResponse, Range, WorkspaceEdit};

use crate::defaults::declare;
use crate::served::Served;

const MAIN: &str =
    "use { Repo as Store } from \"a.spec\"\n\nbehavior login \"Login\" {\n  ports [Repo]\n}\n";

fn served() -> Served {
    Served::new(&[("a.spec", "port Repo \"Repo\" {\n}\n"), ("main.spec", MAIN)])
        .extension("@t/defaults", declare)
        .open(&["main.spec"])
}

fn range(r: Range) -> String {
    format!(
        "{}:{}-{}:{}",
        r.start.line, r.start.character, r.end.line, r.end.character
    )
}

fn name(uri: &tower_lsp::lsp_types::Url) -> String {
    uri.path().rsplit('/').next().unwrap().to_string()
}

fn located(found: Option<Vec<Location>>) -> Vec<String> {
    let mut out: Vec<String> = found
        .unwrap()
        .into_iter()
        .map(|l| format!("{} {}", name(&l.uri), range(l.range)))
        .collect();
    out.sort();
    out
}

fn edited(edit: WorkspaceEdit) -> Vec<String> {
    let mut out: Vec<String> = edit
        .changes
        .unwrap()
        .into_iter()
        .flat_map(|(uri, edits)| {
            edits
                .into_iter()
                .map(move |e| format!("{} {}", name(&uri), range(e.range)))
        })
        .collect();
    out.sort();
    out
}

#[spec(
    behavior = "find_all_references",
    verify = "find-refs include a use binding's imported name"
)]
fn references_show_a_use_binding() {
    let served = served();
    let found = located(answers::references(
        &served.state(),
        &served.uri("main.spec"),
        Position::new(0, 7),
        true,
    ));
    assert_eq!(
        found,
        ["a.spec 0:5-0:9", "main.spec 0:6-0:10", "main.spec 3:9-3:13"]
    );
}

#[spec(
    behavior = "prepare_rename",
    verify = "prepare rename answers on a use binding's imported name"
)]
fn prepare_rename_and_rename_answer_on_a_use_binding() {
    let served = served();
    let (state, uri) = (served.state(), served.uri("main.spec"));
    let Some(PrepareRenameResponse::Range(r)) =
        answers::prepare_rename(&state, &uri, Position::new(0, 7)).unwrap()
    else {
        panic!("a range on the binding")
    };
    assert_eq!(range(r), "0:6-0:10");
    // The alias names nothing: no rename there.
    assert!(
        answers::prepare_rename(&state, &uri, Position::new(0, 15))
            .unwrap()
            .is_none()
    );
    let edits = edited(
        answers::rename(&state, &uri, Position::new(3, 10), "Depot")
            .unwrap()
            .unwrap(),
    );
    assert!(
        edits.contains(&"main.spec 0:6-0:10".to_string()),
        "{edits:?}"
    );
    assert!(
        !edits.iter().any(|e| e.starts_with("main.spec 0:14")),
        "{edits:?}"
    );
}
