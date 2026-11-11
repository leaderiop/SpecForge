//! Editor renames whose files change beside them (ADR 0289, ADR 0346 D7): another entity renamed in the same source
//! file, typing elsewhere in it, a redo after a save, a rename answered and never applied, one store placed in two
//! source files. A source file holds a rename where the names it wrote still read, whatever else changed around them.

use specforge_lsp::answers;
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::Position;

use crate::rename::{FACT_KEY, apply_to_buffers, fact_named};
use crate::rename_pending::{buffer, save};
use crate::rename_undo::{SOURCE, SPEC, composed, landed, said, shown};
use crate::served::Served;

/// Rename the entity declared at `line`:7 of the declaring file to `name`, applied to the buffers.
fn rename(served: &mut Served, line: u32, name: &str) {
    let uri = served.uri(SPEC);
    let edit = answers::rename(&served.state(), &uri, Position::new(line, 7), name)
        .unwrap()
        .expect("an edit");
    apply_to_buffers(served, edit);
}

/// `declared` in the declaring file, and each of `files` beginning `line`, where one fact of the store covers each of
/// `places` (a name and its column on line 1).
fn served_with(declared: &str, line: &str, places: &[(&str, usize)], files: &[&str]) -> Served {
    use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, stored_with};
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join(SPEC), declared).unwrap();
    let mut digests = std::collections::BTreeMap::new();
    let mut facts = Vec::new();
    for file in files {
        let text = format!("{line}\ntest('adds', () => {{}});\n");
        std::fs::write(dir.path().join(file), &text).unwrap();
        digests.insert(
            file.to_string(),
            specforge_ops::revision::of_bytes(text.as_bytes()),
        );
        let mut named = serde_json::Map::new();
        for (name, col) in places {
            named.insert(
                name.to_string(),
                serde_json::json!([{"file": file, "start_line": 1, "start_col": col,
                                    "end_line": 1, "end_col": col + name.len()}]),
            );
        }
        let covers: Vec<&str> = places.iter().map(|(name, _)| *name).collect();
        facts.push(
            serde_json::json!({"kind": "test", "id": format!("js:{file}#adds"),
            "fields": {"file": file, "covers": covers}, "named_at": {"covers": named}}),
        );
    }
    stored_with(
        dir.path(),
        "node",
        FACT_KEY,
        serde_json::Value::Array(facts),
        digests,
    );
    Served::served_by(dir, Collected::new().runtime(), &COLLECTED_EXTENSIONS).open(&[SPEC])
}

/// `T1` and `U1`, one fact naming both in `a.test.js`.
fn two_named() -> Served {
    served_with(
        "truth T1 \"One\" {\n}\n\ntruth U1 \"Two\" {\n}\n",
        "// covers T1 U1",
        &[("T1", 11), ("U1", 14)],
        &[SOURCE],
    )
}

/// `T1` renamed to `T2`, then `U1`, another entity named in the same source file, to `U2`: each rename's line holds,
/// shown and landed.
#[spec(
    behavior = "land_rename_ledger",
    verify = "an editor rename's stale line holds while its names read in the source, whatever else changed there"
)]
fn a_rename_of_another_entity_in_the_same_source_keeps_the_first_line() {
    let mut served = two_named();
    rename(&mut served, 0, "T2");
    rename(&mut served, 3, "U2");
    assert!(buffer(&served, SOURCE).starts_with("// covers T2 U2\n"));

    let expected = said(&[("T1", "T2"), ("U1", "U2")], &[("T2", "T1"), ("U2", "U1")]);
    assert_eq!(shown(&served), expected);
    save(&mut served, &[SPEC, SOURCE]);
    assert_eq!(landed(&served), expected);
}

/// `T1` renamed to `T2`, then a test typed below in the source file: the line holds.
#[spec(
    behavior = "land_rename_ledger",
    verify = "an editor rename's stale line holds while its names read in the source, whatever else changed there"
)]
fn typing_elsewhere_in_the_source_keeps_the_line() {
    let mut served = fact_named();
    rename(&mut served, 0, "T2");
    let text = buffer(&served, SOURCE) + "test('more', () => {});\n";
    served.edit(SOURCE, &text);

    let expected = said(&[("T1", "T2")], &[("T2", "T1")]);
    assert_eq!(shown(&served), expected);
    save(&mut served, &[SPEC, SOURCE]);
    assert_eq!(landed(&served), expected);
}

/// `T1` renamed to `T2`, then the name typed over in the source (`T2x`, which no rename wrote): the source no longer
/// holds the rename there, so no line.
#[spec(
    behavior = "land_rename_ledger",
    verify = "an editor rename's stale line holds while its names read in the source, whatever else changed there"
)]
fn a_name_typed_over_in_the_source_drops_its_line() {
    let mut served = fact_named();
    rename(&mut served, 0, "T2");
    let text = buffer(&served, SOURCE).replacen("covers T2", "covers T2x", 1);
    served.edit(SOURCE, &text);

    let expected = said(&[], &[("T2", "T1")]);
    assert_eq!(shown(&served), expected);
    save(&mut served, &[SPEC, SOURCE]);
    assert_eq!(landed(&served), expected);
}

/// `T1 → T2 → T3`, undone once in both files and saved (the first lands), then redone in both and saved.
#[spec(
    behavior = "land_rename_ledger",
    verify = "editor renames redone after a save land what the redo says"
)]
fn a_redo_after_a_save_lands_the_rest() {
    let (mut served, steps) = composed();
    served.edit(SPEC, &steps.spec[1]);
    served.edit(SOURCE, &steps.source[1]);
    save(&mut served, &[SPEC, SOURCE]);
    assert_eq!(landed(&served), said(&[("T1", "T2")], &[("T2", "T1")]));

    served.edit(SPEC, &steps.spec[2]);
    served.edit(SOURCE, &steps.source[2]);
    save(&mut served, &[SPEC, SOURCE]);

    // Today: the chain was over at the first save; the redo lands nothing.
    assert_eq!(landed(&served), said(&[("T1", "T2")], &[("T2", "T1")]));
}

/// `T1` renamed to `T2`, then `T2` to `T3` answered and never applied; the declaring file edited otherwise (the title),
/// then `T3` typed by hand: no rename wrote that edit, so no move lands.
#[spec(
    behavior = "land_rename_ledger",
    verify = "an editor rename never applied is dropped at the first change of its declaring file that is not its edit"
)]
fn a_rename_never_applied_lands_no_move_for_a_name_typed_later() {
    let mut served = fact_named();
    rename(&mut served, 0, "T2");
    let uri = served.uri(SPEC);
    answers::rename(&served.state(), &uri, Position::new(0, 7), "T3")
        .unwrap()
        .expect("answered, never applied");
    let spec = buffer(&served, SPEC).replacen("\"One\"", "\"Uno\"", 1);
    served.edit(SPEC, &spec);
    served.edit(SPEC, &spec.replacen("truth T2 ", "truth T3 ", 1));

    save(&mut served, &[SPEC, SOURCE]);

    // Today: the rename never applied matches the name typed, and its move lands.
    assert_eq!(landed(&served), said(&[("T1", "T2")], &[("T3", "T1")]));
}

/// One store placing `T1` in `a.test.js` and in `b.test.js`, renamed `T1 → T2 → T3`. One stale line names one name
/// for both files: with `b.test.js` undone once it is the first rename's, undone twice there is none, though
/// `a.test.js` still says `T3` (W198 there until the next collect).
#[spec(
    behavior = "land_rename_ledger",
    verify = "a stale line about two source files is the line of the earliest rename either holds"
)]
fn a_store_in_two_files_undone_in_one_reads_the_earlier_step() {
    let mut served = served_with(
        "truth T1 \"One\" {\n}\n",
        "// covers T1",
        &[("T1", 11)],
        &[SOURCE, "b.test.js"],
    );
    let b0 = buffer_or_disk(&served, "b.test.js");
    rename(&mut served, 0, "T2");
    let b1 = buffer(&served, "b.test.js");
    rename(&mut served, 0, "T3");

    served.edit("b.test.js", &b1);
    assert_eq!(shown(&served), said(&[("T1", "T2")], &[("T3", "T1")]));
    served.edit("b.test.js", &b0);
    assert_eq!(shown(&served), said(&[], &[("T3", "T1")]));

    save(&mut served, &[SPEC, SOURCE, "b.test.js"]);
    assert_eq!(landed(&served), said(&[], &[("T3", "T1")]));
}

fn buffer_or_disk(served: &Served, file: &str) -> String {
    served.state().files().document(file).map_or_else(
        || std::fs::read_to_string(served.root().join(file)).unwrap(),
        |d| d.text().to_string(),
    )
}
