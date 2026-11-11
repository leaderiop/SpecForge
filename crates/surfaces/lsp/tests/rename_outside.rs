//! Editor renames beside what changes outside them (ADR 0289, ADR 0346 D7): text typed on the name's own line, a
//! rename another surface writes (the CLI, MCP, another client) while the files stay open, an entity declared by hand
//! under the name a rename left, and a scope and what it scopes renamed one after the other.

use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_lsp::answers;
use specforge_lsp::changes::Change;
use specforge_ops::asking::Asking;
use specforge_ops::authoring::apply::Origin;
use specforge_ops::authoring::rename as renaming;
use specforge_ops::read::navigate::Navigator;
use specforge_ops::view::ProjectView;
use specforge_ops::{Caller, Containment};
use specforge_project::testing::{
    COLLECTED_EXTENSIONS, Collected, SCOPED_EXTENSION, scoped_kinds, stored_with,
};
use specforge_test_macros::test as spec;
use specforge_wasm::testing::InProcessRuntime;
use tower_lsp::lsp_types::{FileChangeType, FileEvent, Position};

use crate::rename::{FACT_KEY, FACT_STORE, apply_to_buffers, fact_named};
use crate::rename_history::rename;
use crate::rename_pending::{buffer, save};
use crate::rename_undo::{SOURCE, SPEC, landed, said, shown};
use crate::served::Served;

const LEDGER: &str = "specforge-harness";

/// The editor's watcher reports that `files` changed on disk.
fn watched(served: &mut Served, files: &[&str]) {
    let events = files
        .iter()
        .map(|file| FileEvent {
            uri: served.uri(file),
            typ: FileChangeType::CHANGED,
        })
        .collect();
    served.apply(Change::Watched(events));
}

fn disk(served: &Served, file: &str) -> String {
    std::fs::read_to_string(served.root().join(file)).unwrap()
}

/// `from` renamed to `to` by the CLI over what the session holds, as an editor sees it: the files it wrote change on
/// disk, then the open buffers reload what the disk says.
fn renamed_outside(served: &mut Served, from: &str, to: &str) {
    let project = served.with_session(|session| session.project().clone());
    let unbounded = Containment::unbounded();
    renaming::run(
        &Navigator::new(ProjectView::of(&project)),
        from,
        to,
        false,
        &Origin::cli(false),
        Caller::new(&unbounded, Asking::Nobody),
    )
    .expect("renamed");
    watched(served, &[SPEC, SOURCE, LEDGER, FACT_STORE]);
    for file in [SPEC, SOURCE] {
        let text = disk(served, file);
        served.edit(file, &text);
    }
}

/// `T1` renamed to `T2`, then text typed after the name on its line (`// covers T2, T5`): nothing typed there can
/// extend the name, so it still reads and the line holds.
#[spec(
    behavior = "land_rename_ledger",
    verify = "an editor rename's stale line holds while its names read in the source, whatever else changed there"
)]
fn text_typed_after_the_name_on_its_line_keeps_the_line() {
    let mut served = fact_named();
    rename(&mut served, 0, "T2");
    let text = buffer(&served, SOURCE).replacen("// covers T2\n", "// covers T2, T5\n", 1);
    served.edit(SOURCE, &text);

    let expected = said(&[("T1", "T2")], &[("T2", "T1")]);
    assert_eq!(shown(&served), expected);
    save(&mut served, &[SPEC, SOURCE]);
    assert_eq!(landed(&served), expected);
}

/// `T1` renamed to `T2` and saved, the files left open; then the CLI renames `T2` to `T7`, and later back to `T1`. The
/// editor's tree no longer reads as the ledger: it shows nothing, and the save after the rename back lands no second
/// move over the CLI's.
#[spec(
    behavior = "land_rename_ledger",
    verify = "a ledger write from outside the editor ends the editor renames it touches"
)]
fn a_rename_written_outside_while_open_ends_the_tree() {
    let mut served = fact_named();
    rename(&mut served, 0, "T2");
    save(&mut served, &[SPEC, SOURCE]);
    watched(&mut served, &[LEDGER]);

    renamed_outside(&mut served, "T2", "T7");
    assert_eq!(shown(&served), said(&[], &[]));

    renamed_outside(&mut served, "T7", "T1");
    let outside = landed(&served);
    assert_eq!(shown(&served), said(&[], &[]));
    save(&mut served, &[SPEC, SOURCE]);
    assert_eq!(landed(&served), outside);
}

/// `T1` renamed to `T2` and saved, then an entity declared by hand under the old name (`T1`) beside `T2`, in the
/// declaring file or another: the rename still holds, and no move lands onto the new entity.
#[spec(
    behavior = "land_rename_ledger",
    verify = "an entity declared by hand under the name an editor rename left takes nothing of it"
)]
fn an_entity_declared_by_hand_under_the_old_name_takes_nothing() {
    for file in [SPEC, "other.spec"] {
        let mut served = fact_named();
        rename(&mut served, 0, "T2");
        save(&mut served, &[SPEC, SOURCE]);
        let renamed = landed(&served);
        assert_eq!(renamed, said(&[("T1", "T2")], &[("T2", "T1")]));

        let text = if file == SPEC {
            buffer(&served, SPEC)
        } else {
            String::new()
        };
        served.write(file, &text);
        let declared = format!("{text}\ntruth T1 \"New\" {{\n}}\n");
        if file == SPEC {
            served.edit(file, &declared);
        } else {
            let uri = served.uri(file);
            served.open_document(&uri, &declared);
        }
        assert_eq!(shown(&served), said(&[], &[]), "{file}");
        save(&mut served, &[file]);
        assert_eq!(landed(&served), renamed, "{file}");
    }
}

const MAIN: &str = "main.spec";

/// The workstream `ws` and its milestone `ws/m1` in `main.spec`, a fact naming `ws/m1` in the source file.
fn scoped() -> Served {
    fn kinds() -> ContributionsBuilder {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(SCOPED_EXTENSION, "1.0.0"));
        scoped_kinds(&mut c);
        c
    }
    let dir = tempfile::TempDir::new().unwrap();
    let source = "// covers ws/m1\ntest('adds', () => {});\n";
    let specs = "workstream ws \"W\" {}\n\nmilestone m1 \"M\" {\n  workstream ws\n}\n";
    std::fs::write(dir.path().join(MAIN), specs).unwrap();
    std::fs::write(dir.path().join(SOURCE), source).unwrap();
    let place = serde_json::json!({"file": SOURCE, "start_line": 1, "start_col": 11, "end_line": 1, "end_col": 16});
    stored_with(
        dir.path(),
        "node",
        FACT_KEY,
        serde_json::json!([{"kind": "test", "id": "js:a.test.js#adds",
            "fields": {"file": SOURCE, "covers": ["ws/m1"]}, "named_at": {"covers": {"ws/m1": [place]}}}]),
        [(
            SOURCE.to_string(),
            specforge_ops::revision::of_bytes(source.as_bytes()),
        )]
        .into(),
    );
    let mut enabled = COLLECTED_EXTENSIONS.to_vec();
    enabled.push(SCOPED_EXTENSION);
    let runtime = Collected::new().over(InProcessRuntime::new().with(kinds));
    Served::served_by(dir, runtime, &enabled).open(&[MAIN])
}

/// Rename what `main.spec` declares at `line`:`col` to `name`, applied to the buffers.
fn rename_at(served: &mut Served, line: u32, col: u32, name: &str) {
    let uri = served.uri(MAIN);
    let edit = answers::rename(&served.state(), &uri, Position::new(line, col), name)
        .unwrap()
        .expect("an edit");
    apply_to_buffers(served, edit);
}

/// The texts of `main.spec` and of the source file.
fn texts(served: &Served) -> [String; 2] {
    let text = |file: &str| {
        served.state().files().document(file).map_or_else(
            || disk(served, file),
            |document| document.text().to_string(),
        )
    };
    [text(MAIN), text(SOURCE)]
}

/// Both files back to `texts`, as the editor's undo leaves them, and saved.
fn undone_to(served: &mut Served, texts: &[String; 2]) {
    served.edit(MAIN, &texts[0]);
    served.edit(SOURCE, &texts[1]);
    save(served, &[MAIN, SOURCE]);
}

/// The milestone renamed `m1 → m2`, then its workstream `ws → wz`, both saved at once, then each undone in the order
/// the editor undoes them: what lands is what one rename after the other leaves, the path between kept, and the stale
/// line follows each step.
#[spec(
    behavior = "land_rename_ledger",
    verify = "editor renames of a scope and of what it scopes compose as one history"
)]
fn a_milestone_then_its_workstream_renamed_compose_as_one_history() {
    let mut served = scoped();
    let first = texts(&served);
    rename_at(&mut served, 2, 10, "m2");
    let renamed = texts(&served);
    rename_at(&mut served, 0, 11, "wz");
    assert!(buffer(&served, SOURCE).starts_with("// covers wz/m2\n"));

    save(&mut served, &[MAIN, SOURCE]);
    let both = said(&[("ws/m1", "wz/m2")], &[("wz", "ws"), ("wz/m2", "ws/m1")]);
    assert_eq!(landed(&served), both);

    undone_to(&mut served, &renamed);
    let moves = [("ws", "wz"), ("ws/m2", "ws/m1"), ("ws/m2", "wz/m2")];
    assert_eq!(landed(&served), said(&[("ws/m1", "ws/m2")], &moves));

    undone_to(&mut served, &first);
    let moves = [("ws", "wz"), ("ws/m1", "ws/m2"), ("ws/m1", "wz/m2")];
    assert_eq!(landed(&served), said(&[], &moves));
}

/// The workstream renamed `ws → wz`, then its milestone `m1 → m2`, both saved at once, then each undone in the order
/// the editor undoes them.
#[spec(
    behavior = "land_rename_ledger",
    verify = "editor renames of a scope and of what it scopes compose as one history"
)]
fn a_workstream_then_its_milestone_renamed_compose_as_one_history() {
    let mut served = scoped();
    let first = texts(&served);
    rename_at(&mut served, 0, 11, "wz");
    let renamed = texts(&served);
    rename_at(&mut served, 2, 10, "m2");
    assert!(buffer(&served, SOURCE).starts_with("// covers wz/m2\n"));

    save(&mut served, &[MAIN, SOURCE]);
    let both = said(&[("ws/m1", "wz/m2")], &[("wz", "ws"), ("wz/m2", "ws/m1")]);
    assert_eq!(landed(&served), both);

    undone_to(&mut served, &renamed);
    let moves = [("wz", "ws"), ("wz/m1", "ws/m1"), ("wz/m1", "wz/m2")];
    assert_eq!(landed(&served), said(&[("ws/m1", "wz/m1")], &moves));

    undone_to(&mut served, &first);
    let moves = [("ws", "wz"), ("ws/m1", "wz/m1"), ("ws/m1", "wz/m2")];
    assert_eq!(landed(&served), said(&[], &moves));
}
