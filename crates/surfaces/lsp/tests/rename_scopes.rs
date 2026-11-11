//! Editor renames of a scope and of what it scopes declared in different files (ADR 0346 D7): a milestone in one file,
//! its phases in others, a test of each phase whose harness value the ledger records. Renamed one after the other
//! they are one history, whichever files declare them, and a rename touching two histories joins them.

use specforge_extension_sdk::DeclaredCategory;
use specforge_lsp::answers;
use specforge_project::testing::GSD_EXTENSION;
use specforge_project::testing::gsd::{gsd_history_passes, gsd_history_wire};
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::Position;

use crate::rename::apply_to_buffers;
use crate::rename_pending::{ledger, save};
use crate::served::Served;

const FILES: [&str; 4] = ["m.spec", "p.spec", "q.spec", "u.spec"];

/// The ledger line recording `pass` as the `result` of the test at `path`.
fn result(path: &str) -> String {
    let digest =
        specforge_project::testing::revision_of(format!("{path}\0result\0\"pass\"").as_bytes());
    format!("{path} result {}\n", digest.trim_start_matches("sha256:"))
}

/// `milestone v1.0` in `m.spec`, its `phase 03` in `p.spec` and `phase 05` in `q.spec`, a test of each in `u.spec`
/// whose `result` the ledger records.
fn served() -> Served {
    let ledger = format!(
        "# specforge-harness 1\n{}{}",
        result("v1.0/03/U1"),
        result("v1.0/05/U5")
    );
    let tests = "uat_test U1 \"One\" {\n  phase v1.0/03\n  result \"pass\"\n}\n\n\
                 uat_test U5 \"Five\" {\n  phase v1.0/05\n  result \"pass\"\n}\n";
    Served::new(&[
        ("m.spec", "milestone v1.0 \"One\" {}\n"),
        ("p.spec", "phase 03 \"Three\" {\n  milestone v1.0\n}\n"),
        ("q.spec", "phase 05 \"Five\" {\n  milestone v1.0\n}\n"),
        ("u.spec", tests),
        ("specforge-harness", &ledger),
    ])
    .extension(GSD_EXTENSION, |c| {
        c.raw_category(DeclaredCategory::Entities, gsd_history_wire());
        c.raw_category(DeclaredCategory::Passes, gsd_history_passes());
    })
    .open(&FILES)
}

/// Rename what `file` declares on its first line to `name`, applied to the buffers.
fn rename(served: &mut Served, file: &str, name: &str) {
    let uri = served.uri(file);
    let col = if file == "m.spec" { 10 } else { 6 };
    let edit = answers::rename(&served.state(), &uri, Position::new(0, col), name)
        .unwrap()
        .expect("an edit");
    apply_to_buffers(served, edit);
}

fn texts(served: &Served) -> Vec<String> {
    let state = served.state();
    let files = state.files();
    FILES
        .iter()
        .map(|file| files.document(file).expect("open").text().to_string())
        .collect()
}

/// Every file back to `texts`, as the editor's undo leaves them, and saved.
fn undone_to(served: &mut Served, texts: &[String]) {
    for (file, text) in FILES.iter().zip(texts) {
        served.edit(file, text);
    }
    save(served, &FILES);
}

/// What the ledger says: its move records, as (new, old), and its value lines, each as [`result`] writes it.
type Held = (Vec<(String, String)>, Vec<String>);

fn held(served: &Served) -> Held {
    let ledger = ledger(served);
    let moved = ledger
        .lines()
        .filter_map(|l| Some((l.subject.clone(), l.moved_from()?.to_string())))
        .collect();
    let values = ledger
        .lines()
        .filter(|l| l.moved_from().is_none())
        .map(|l| format!("{} {} {}\n", l.subject, l.key, l.digest))
        .collect();
    (moved, values)
}

fn says(moved: &[(&str, &str)], results: &[&str]) -> Held {
    let mut moved: Vec<(String, String)> = moved
        .iter()
        .map(|(new, old)| (new.to_string(), old.to_string()))
        .collect();
    moved.sort();
    let mut values: Vec<String> = results.iter().map(|path| result(path)).collect();
    values.sort();
    (moved, values)
}

/// After both renames, whichever came first: the milestone at `v1.1`, phase `03` at `04`.
fn both() -> Held {
    says(
        &[
            ("v1.1", "v1.0"),
            ("v1.1/04", "v1.0/03"),
            ("v1.1/04/U1", "v1.0/03/U1"),
            ("v1.1/05", "v1.0/05"),
            ("v1.1/05/U5", "v1.0/05/U5"),
        ],
        &["v1.1/04/U1", "v1.1/05/U5"],
    )
}

/// The phase renamed `03 → 04` in `p.spec`, then its milestone `v1.0 → v1.1` in `m.spec`, saved at once, then each
/// undone in the order the editor undoes them: each step lands, and the tests' results follow every move.
#[spec(
    behavior = "land_rename_ledger",
    verify = "editor renames of a scope and of what it scopes compose as one history"
)]
fn a_phase_then_its_milestone_in_other_files_compose_as_one_history() {
    let mut served = served();
    let first = texts(&served);
    rename(&mut served, "p.spec", "04");
    let renamed = texts(&served);
    rename(&mut served, "m.spec", "v1.1");

    save(&mut served, &FILES);
    assert_eq!(held(&served), both());

    undone_to(&mut served, &renamed);
    let moved = [
        ("v1.0", "v1.1"),
        ("v1.0/04", "v1.0/03"),
        ("v1.0/04", "v1.1/04"),
        ("v1.0/04/U1", "v1.0/03/U1"),
        ("v1.0/04/U1", "v1.1/04/U1"),
        ("v1.0/05", "v1.1/05"),
        ("v1.0/05/U5", "v1.1/05/U5"),
    ];
    assert_eq!(held(&served), says(&moved, &["v1.0/04/U1", "v1.0/05/U5"]));

    undone_to(&mut served, &first);
    let moved = [
        ("v1.0", "v1.1"),
        ("v1.0/03", "v1.0/04"),
        ("v1.0/03", "v1.1/04"),
        ("v1.0/03/U1", "v1.0/04/U1"),
        ("v1.0/03/U1", "v1.1/04/U1"),
        ("v1.0/05", "v1.1/05"),
        ("v1.0/05/U5", "v1.1/05/U5"),
    ];
    assert_eq!(held(&served), says(&moved, &["v1.0/03/U1", "v1.0/05/U5"]));
}

/// The milestone renamed `v1.0 → v1.1` in `m.spec`, then its phase `03 → 04` in `p.spec`, saved at once, then each
/// undone in the order the editor undoes them.
#[spec(
    behavior = "land_rename_ledger",
    verify = "editor renames of a scope and of what it scopes compose as one history"
)]
fn a_milestone_then_its_phase_in_other_files_compose_as_one_history() {
    let mut served = served();
    let first = texts(&served);
    rename(&mut served, "m.spec", "v1.1");
    let renamed = texts(&served);
    rename(&mut served, "p.spec", "04");

    save(&mut served, &FILES);
    assert_eq!(held(&served), both());

    undone_to(&mut served, &renamed);
    let moved = [
        ("v1.1", "v1.0"),
        ("v1.1/03", "v1.0/03"),
        ("v1.1/03", "v1.1/04"),
        ("v1.1/03/U1", "v1.0/03/U1"),
        ("v1.1/03/U1", "v1.1/04/U1"),
        ("v1.1/05", "v1.0/05"),
        ("v1.1/05/U5", "v1.0/05/U5"),
    ];
    assert_eq!(held(&served), says(&moved, &["v1.1/03/U1", "v1.1/05/U5"]));

    undone_to(&mut served, &first);
    let moved = [
        ("v1.0", "v1.1"),
        ("v1.0/03", "v1.1/03"),
        ("v1.0/03", "v1.1/04"),
        ("v1.0/03/U1", "v1.1/03/U1"),
        ("v1.0/03/U1", "v1.1/04/U1"),
        ("v1.0/05", "v1.1/05"),
        ("v1.0/05/U5", "v1.1/05/U5"),
    ];
    assert_eq!(held(&served), says(&moved, &["v1.0/03/U1", "v1.0/05/U5"]));
}

/// Phase `03 → 04` and phase `05 → 06`, two histories (neither moves what the other does), then their milestone
/// `v1.0 → v1.1`, which moves both: one history, saved at once, then undone in the order the editor undoes them.
#[spec(
    behavior = "land_rename_ledger",
    verify = "an editor rename touching two histories of renames joins them into one"
)]
fn a_milestone_renamed_after_two_of_its_phases_joins_their_histories() {
    let mut served = served();
    let first = texts(&served);
    rename(&mut served, "p.spec", "04");
    let one = texts(&served);
    rename(&mut served, "q.spec", "06");
    let two = texts(&served);
    rename(&mut served, "m.spec", "v1.1");

    save(&mut served, &FILES);
    let moved = [
        ("v1.1", "v1.0"),
        ("v1.1/04", "v1.0/03"),
        ("v1.1/04/U1", "v1.0/03/U1"),
        ("v1.1/06", "v1.0/05"),
        ("v1.1/06/U5", "v1.0/05/U5"),
    ];
    assert_eq!(held(&served), says(&moved, &["v1.1/04/U1", "v1.1/06/U5"]));

    undone_to(&mut served, &two);
    let moved = [
        ("v1.0", "v1.1"),
        ("v1.0/04", "v1.0/03"),
        ("v1.0/04", "v1.1/04"),
        ("v1.0/04/U1", "v1.0/03/U1"),
        ("v1.0/04/U1", "v1.1/04/U1"),
        ("v1.0/06", "v1.0/05"),
        ("v1.0/06", "v1.1/06"),
        ("v1.0/06/U5", "v1.0/05/U5"),
        ("v1.0/06/U5", "v1.1/06/U5"),
    ];
    assert_eq!(held(&served), says(&moved, &["v1.0/04/U1", "v1.0/06/U5"]));

    undone_to(&mut served, &one);
    let moved = [
        ("v1.0", "v1.1"),
        ("v1.0/04", "v1.0/03"),
        ("v1.0/04", "v1.1/04"),
        ("v1.0/04/U1", "v1.0/03/U1"),
        ("v1.0/04/U1", "v1.1/04/U1"),
        ("v1.0/05", "v1.0/06"),
        ("v1.0/05", "v1.1/06"),
        ("v1.0/05/U5", "v1.0/06/U5"),
        ("v1.0/05/U5", "v1.1/06/U5"),
    ];
    assert_eq!(held(&served), says(&moved, &["v1.0/04/U1", "v1.0/05/U5"]));

    undone_to(&mut served, &first);
    let moved = [
        ("v1.0", "v1.1"),
        ("v1.0/03", "v1.0/04"),
        ("v1.0/03", "v1.1/04"),
        ("v1.0/03/U1", "v1.0/04/U1"),
        ("v1.0/03/U1", "v1.1/04/U1"),
        ("v1.0/05", "v1.0/06"),
        ("v1.0/05", "v1.1/06"),
        ("v1.0/05/U5", "v1.0/06/U5"),
        ("v1.0/05/U5", "v1.1/06/U5"),
    ];
    assert_eq!(held(&served), says(&moved, &["v1.0/03/U1", "v1.0/05/U5"]));
}

/// The milestone renamed, then its phase, saved; then the milestone, the earlier rename, undone by hand with the
/// phase's rename kept. No history holds those texts: nothing is shown, and the save lands nothing, so no move no
/// rename made and no harness value dropped.
#[spec(
    behavior = "land_rename_ledger",
    verify = "an earlier editor rename undone by hand before a later one it led to lands nothing"
)]
fn an_earlier_rename_undone_by_hand_before_a_later_one_lands_nothing() {
    let mut served = served();
    rename(&mut served, "m.spec", "v1.1");
    rename(&mut served, "p.spec", "04");
    save(&mut served, &FILES);
    assert_eq!(held(&served), both());

    for (file, text) in FILES.iter().zip(texts(&served)) {
        served.edit(file, &text.replace("v1.1", "v1.0"));
    }
    assert!(served.state().pending().shown().is_empty());
    save(&mut served, &FILES);
    assert_eq!(held(&served), both());
}
