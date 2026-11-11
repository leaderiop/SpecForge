//! A pending editor rename and a later one (ADR 0289, ADR 0346 D7): the later composes with the earlier only once its
//! own edit is in the buffers, it is matched by the entity's path, not its name, and the buffer typed back to the
//! first rename's old name undoes both.

use specforge_lsp::answers;
use specforge_lsp::changes::Change;
use specforge_project::harness::{Ledger, Line};
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{FileChangeType, FileEvent, Position};

use crate::rename::{FACT_KEY, FACT_STORE, apply_to_buffers, fact_named};
use crate::served::Served;

pub(crate) fn buffer(served: &Served, file: &str) -> String {
    served
        .state()
        .files()
        .document(file)
        .unwrap()
        .text()
        .to_string()
}

/// Save `files` as their buffers hold them, and the watch events of the save.
pub(crate) fn save(served: &mut Served, files: &[&str]) {
    for file in files {
        served.write(file, &buffer(served, file));
    }
    let events = files
        .iter()
        .map(|file| FileEvent {
            uri: served.uri(file),
            typ: FileChangeType::CHANGED,
        })
        .collect();
    served.apply(Change::Watched(events));
}

pub(crate) fn ledger(served: &Served) -> Ledger {
    Ledger::parse(
        &std::fs::read_to_string(served.root().join("specforge-harness")).unwrap_or_default(),
    )
    .ledger
}

/// A second rename answered and never applied (a preview cancelled, an edit refused) leaves the first as it was: it
/// shows, and saved, it lands as itself.
#[spec(
    behavior = "land_rename_ledger",
    verify = "a later editor rename never applied leaves the earlier one to show and land alone"
)]
fn a_second_rename_never_applied_leaves_the_first_to_land_alone() {
    let mut served = fact_named();
    let uri = served.uri("truths.spec");
    let store = std::fs::read(served.root().join(FACT_STORE)).unwrap();
    let first = answers::rename(&served.state(), &uri, Position::new(0, 7), "T2")
        .unwrap()
        .expect("an edit");
    apply_to_buffers(&mut served, first);

    answers::rename(&served.state(), &uri, Position::new(0, 7), "T3")
        .unwrap()
        .expect("the second rename is answered");

    let open = served.state().pending().open();
    assert_eq!(open.len(), 2);
    assert!(open[0].is_showing(), "the first still shows: {open:?}");
    assert!(!open[1].is_showing());
    save(&mut served, &["truths.spec", "a.test.js"]);
    let ledger = ledger(&served);
    assert!(
        ledger.contains(&Line::stale("node", FACT_KEY, &store, "covers", "T1", "T2")),
        "{}",
        ledger.canonical()
    );
    assert!(
        ledger.contains(&Line::moved("T2", "T1")),
        "{}",
        ledger.canonical()
    );
    assert_eq!(ledger.lines().filter_map(Line::stale_of).count(), 1);
}

/// Two renames composed, then the buffers typed back to the first's old name: both are undone, none shows; saved,
/// nothing lands and both are over.
#[spec(
    behavior = "land_rename_ledger",
    verify = "a chain of editor renames typed back to its first name is undone whole"
)]
fn a_chain_typed_back_to_its_first_name_is_undone_whole() {
    let mut served = fact_named();
    let uri = served.uri("truths.spec");
    let (spec, source) = (
        buffer_or_disk(&served, "truths.spec"),
        disk(&served, "a.test.js"),
    );
    for name in ["T2", "T3"] {
        let edit = answers::rename(&served.state(), &uri, Position::new(0, 7), name)
            .unwrap()
            .expect("an edit");
        apply_to_buffers(&mut served, edit);
    }
    assert_eq!(
        served
            .state()
            .pending()
            .open()
            .iter()
            .filter(|p| p.is_showing())
            .count(),
        1
    );

    served.edit("a.test.js", &source);
    served.edit("truths.spec", &spec);

    assert!(served.state().pending().shown().is_empty());
    assert!(
        served
            .state()
            .pending()
            .open()
            .iter()
            .all(|p| !p.is_showing()),
        "{:?}",
        served.state().pending().open()
    );
    save(&mut served, &["truths.spec", "a.test.js"]);
    assert_eq!(
        ledger(&served).lines().filter_map(Line::stale_of).count(),
        0
    );
    assert!(served.state().pending().open().is_empty());
}

fn disk(served: &Served, file: &str) -> String {
    std::fs::read_to_string(served.root().join(file)).unwrap()
}

fn buffer_or_disk(served: &Served, file: &str) -> String {
    served
        .state()
        .files()
        .document(file)
        .map_or_else(|| disk(served, file), |d| d.text().to_string())
}

/// Two scoped phases in one file: `v1.0/03` and `v2.0/04`.
const M: &str = "milestone v1.0 \"One\" {}\n\nmilestone v2.0 \"Two\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\nphase 04 \"Four\" {\n  milestone v2.0\n}\n";
const U: &str = "uat_test U1 \"One\" {\n  phase v1.0/03\n  result \"pass\"\n}\n";

fn scoped() -> Served {
    use specforge_extension_sdk::DeclaredCategory;
    use specforge_project::testing::GSD_EXTENSION;
    use specforge_project::testing::gsd::{gsd_history_passes, gsd_history_wire};
    let digest = specforge_project::testing::revision_of("v1.0/03/U1\0result\0\"pass\"".as_bytes());
    let ledger = format!(
        "# specforge-harness 1\nv1.0/03/U1 result {}\n",
        digest.trim_start_matches("sha256:")
    );
    Served::new(&[("m.spec", M), ("u.spec", U), ("specforge-harness", &ledger)])
        .extension(GSD_EXTENSION, |c| {
            c.raw_category(DeclaredCategory::Entities, gsd_history_wire());
            c.raw_category(DeclaredCategory::Passes, gsd_history_passes());
        })
        .open(&["m.spec", "u.spec"])
}

/// The position of the name of the phase declared `phase <name>` in [`M`].
fn phase(name: &str) -> Position {
    let prefix = format!("phase {name} ");
    let line = M.lines().position(|l| l.starts_with(&prefix)).unwrap();
    Position::new(line as u32, 7)
}

/// A rename of `v1.0/03` to `04`, then of `v2.0/04`, the other phase that name now shares: the second renames
/// another entity, so it composes with nothing, and each lands as itself.
#[spec(
    behavior = "land_rename_ledger",
    verify = "a later editor rename composes only with the rename of the entity it renames, matched by path"
)]
fn a_rename_of_another_entity_sharing_a_name_composes_with_nothing() {
    let mut served = scoped();
    let uri = served.uri("m.spec");
    let first = answers::rename(&served.state(), &uri, phase("03"), "04")
        .unwrap()
        .expect("an edit");
    apply_to_buffers(&mut served, first);
    assert!(
        buffer(&served, "u.spec").contains("phase v1.0/04"),
        "{}",
        buffer(&served, "u.spec")
    );

    let at = buffer(&served, "m.spec")
        .lines()
        .position(|l| l.starts_with("phase 04 \"Four\""))
        .unwrap();
    let second = answers::rename(&served.state(), &uri, Position::new(at as u32, 7), "05")
        .unwrap()
        .expect("an edit");
    apply_to_buffers(&mut served, second);
    let showing = served
        .state()
        .pending()
        .open()
        .iter()
        .filter(|p| p.is_showing())
        .count();
    assert_eq!(showing, 2, "{:?}", served.state().pending().open());

    save(&mut served, &["m.spec", "u.spec"]);
    let ledger = ledger(&served);
    for moved in [
        Line::moved("v1.0/04", "v1.0/03"),
        Line::moved("v2.0/05", "v2.0/04"),
    ] {
        assert!(ledger.contains(&moved), "{moved:?}: {}", ledger.canonical());
    }
    assert!(
        ledger.canonical().contains("v1.0/04/U1 result"),
        "{}",
        ledger.canonical()
    );
    assert!(served.state().pending().open().is_empty());
}
