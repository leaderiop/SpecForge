//! Two editor renames composed (`T1 → T2`, then `T2 → T3`), and the buffers taken back in each way an editor can
//! (ADR 0289, ADR 0346 D7): undone in every buffer, in one buffer and not the other, part way, by hand, redone. The
//! editor's undo is per buffer: the declaring file and the source file a fact names the entity in each go back alone.

use specforge_lsp::answers;
use specforge_project::harness::Line;
use tower_lsp::lsp_types::Position;

use crate::rename::{apply_to_buffers, fact_named};
use crate::rename_pending::{buffer, ledger, save};
use crate::served::Served;

const SPEC: &str = "truths.spec";
const SOURCE: &str = "a.test.js";

/// The texts of the declaring file and of the source file at each step: before the renames, after the first, after
/// both.
struct Steps {
    spec: [String; 3],
    source: [String; 3],
}

/// `fact_named` with `T1` renamed to `T2`, then `T2` to `T3`, both applied to the buffers and neither saved.
fn composed() -> (Served, Steps) {
    let mut served = fact_named();
    let uri = served.uri(SPEC);
    let mut spec = vec![buffer_or_disk(&served, SPEC)];
    let mut source = vec![buffer_or_disk(&served, SOURCE)];
    for name in ["T2", "T3"] {
        let edit = answers::rename(&served.state(), &uri, Position::new(0, 7), name)
            .unwrap()
            .expect("an edit");
        apply_to_buffers(&mut served, edit);
        spec.push(buffer(&served, SPEC));
        source.push(buffer(&served, SOURCE));
    }
    assert!(spec[2].starts_with("truth T3 "), "{}", spec[2]);
    assert!(source[2].starts_with("// covers T3\n"), "{}", source[2]);
    let steps = Steps {
        spec: spec.try_into().unwrap(),
        source: source.try_into().unwrap(),
    };
    (served, steps)
}

fn buffer_or_disk(served: &Served, file: &str) -> String {
    served.state().files().document(file).map_or_else(
        || std::fs::read_to_string(served.root().join(file)).unwrap(),
        |d| d.text().to_string(),
    )
}

/// Both buffers saved: the stale lines the ledger holds then, as (name held, name written), and its move records,
/// as (new, old).
fn saved(served: &mut Served) -> (Vec<(String, String)>, Vec<(String, String)>) {
    save(served, &[SPEC, SOURCE]);
    let ledger = ledger(served);
    let stale = ledger
        .lines()
        .filter_map(Line::stale_of)
        .map(|s| (s.held.to_string(), s.now.to_string()))
        .collect();
    let moved = ledger
        .lines()
        .filter_map(|l| Some((l.subject.clone(), l.moved_from()?.to_string())))
        .collect();
    (stale, moved)
}

fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_string(), b.to_string())
}

/// The declaring file undone twice (back to `T1`), the source file left at `T3`.
#[test]
fn undone_in_the_declaring_file_only() {
    let (mut served, steps) = composed();
    served.edit(SPEC, &steps.spec[0]);

    // Today: the declaring file back at the first name undoes both, the source's `T3` with them.
    assert_eq!(saved(&mut served), (vec![], vec![]));
}

/// The source file undone twice (back to `T1`), the declaring file left at `T3`.
#[test]
fn undone_in_the_source_file_only() {
    let (mut served, steps) = composed();
    served.edit(SOURCE, &steps.source[0]);

    // Today: the source file is not read; the stale line lands as though it still said `T3`.
    assert_eq!(
        saved(&mut served),
        (vec![pair("T1", "T3")], vec![pair("T3", "T1")])
    );
}

/// The source file undone once (back to `T2`), the declaring file left at `T3`.
#[test]
fn undone_once_in_the_source_file() {
    let (mut served, steps) = composed();
    served.edit(SOURCE, &steps.source[1]);

    // Today: the stale line lands as the second rename wrote it.
    assert_eq!(
        saved(&mut served),
        (vec![pair("T1", "T3")], vec![pair("T3", "T1")])
    );
}

/// The declaring file undone once (back to `T2`), the source file left at `T3`.
#[test]
fn undone_once_in_the_declaring_file() {
    let (mut served, steps) = composed();
    served.edit(SPEC, &steps.spec[1]);

    // Today: the first rename shows again whole, its stale line with it, though the source says `T3`.
    assert_eq!(
        saved(&mut served),
        (vec![pair("T1", "T2")], vec![pair("T2", "T1")])
    );
}

/// Both files undone to `T1`, then redone to `T3`.
#[test]
fn undone_then_redone() {
    let (mut served, steps) = composed();
    served.edit(SOURCE, &steps.source[0]);
    served.edit(SPEC, &steps.spec[0]);
    served.edit(SOURCE, &steps.source[2]);
    served.edit(SPEC, &steps.spec[2]);

    // Today: undone, the renames are dropped; redone, nothing shows and nothing lands.
    assert_eq!(saved(&mut served), (vec![], vec![]));
}

/// The declaring file typed to a third name by hand, the source file left at `T3`.
#[test]
fn a_third_name_typed_by_hand() {
    let (mut served, steps) = composed();
    served.edit(SPEC, &steps.spec[2].replacen("truth T3 ", "truth T9 ", 1));

    // Today: the composed rename keeps showing, never lands and never ends.
    assert_eq!(saved(&mut served), (vec![], vec![]));
    assert_eq!(served.state().pending().open().len(), 2);
}
