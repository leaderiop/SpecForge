//! Two editor renames composed (`T1 → T2`, then `T2 → T3`), and the buffers taken back in each way an editor can
//! (ADR 0289, ADR 0346 D7): undone in every buffer, in one buffer and not the other, part way, by hand, redone. The
//! editor's undo is per buffer: the declaring file and the source file a fact names the entity in each go back alone.

use specforge_lsp::answers;
use specforge_project::harness::{LedgerChange, Line};
use tower_lsp::lsp_types::{FileChangeType, FileEvent, Position};

use crate::rename::{apply_to_buffers, fact_named};
use crate::rename_pending::{buffer, ledger, save};
use crate::served::Served;
use specforge_test_macros::test as spec;

pub(crate) const SPEC: &str = "truths.spec";
pub(crate) const SOURCE: &str = "a.test.js";

/// The texts of the declaring file and of the source file at each step: before the renames, after the first, after
/// both.
pub(crate) struct Steps {
    pub(crate) spec: [String; 3],
    pub(crate) source: [String; 3],
}

/// `fact_named` with `T1` renamed to `T2`, then `T2` to `T3`, both applied to the buffers and neither saved.
pub(crate) fn composed() -> (Served, Steps) {
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

/// What a ledger says of the renames: its stale lines, as (name held, name written), and its move records, as (new,
/// old).
pub(crate) type Said = (Vec<(String, String)>, Vec<(String, String)>);

fn stale_of<'l>(lines: impl Iterator<Item = &'l Line>) -> Vec<(String, String)> {
    lines
        .filter_map(Line::stale_of)
        .map(|s| (s.held.to_string(), s.now.to_string()))
        .collect()
}

/// What the checks read over the session now: the stale lines and moves the pending renames show, sorted.
pub(crate) fn shown(served: &Served) -> Said {
    let shown = served.state().pending().shown();
    let changes: Vec<&LedgerChange> = shown.iter().map(|overlay| overlay.change()).collect();
    let mut stale = stale_of(changes.iter().flat_map(|c| &c.stale));
    let mut moves: Vec<(String, String)> = changes
        .iter()
        .flat_map(|c| &c.moves)
        .map(|(old, new)| (new.clone(), old.clone()))
        .collect();
    stale.sort();
    moves.sort();
    (stale, moves)
}

/// What the ledger on disk says.
pub(crate) fn landed(served: &Served) -> Said {
    let ledger = ledger(served);
    let moved = ledger
        .lines()
        .filter_map(|l| Some((l.subject.clone(), l.moved_from()?.to_string())))
        .collect();
    (stale_of(ledger.lines()), moved)
}

/// Both buffers saved, and the watch events of the save: what the ledger says then. Nothing is left to show.
fn saved(served: &mut Served) -> Said {
    save(served, &[SPEC, SOURCE]);
    assert!(
        served.state().pending().shown().is_empty(),
        "{:?}",
        served.state().pending().shown()
    );
    landed(served)
}

pub(crate) fn said(stale: &[(&str, &str)], moved: &[(&str, &str)]) -> Said {
    let pairs = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    };
    (pairs(stale), pairs(moved))
}

/// The declaring file undone twice (back to `T1`), the source file left at `T3`: no move (the entity is `T1` again),
/// and the store's `T1` written `T3` in the source.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames show and land, file by file, what the buffers hold however they were undone"
)]
fn undone_in_the_declaring_file_only() {
    let (mut served, steps) = composed();
    served.edit(SPEC, &steps.spec[0]);

    let expected = said(&[("T1", "T3")], &[]);
    assert_eq!(shown(&served), expected);
    assert_eq!(saved(&mut served), expected);
}

/// The source file undone twice (back to `T1`), the declaring file left at `T3`: the entity moved, and the source
/// holds what was collected: no stale line.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames show and land, file by file, what the buffers hold however they were undone"
)]
fn undone_in_the_source_file_only() {
    let (mut served, steps) = composed();
    served.edit(SOURCE, &steps.source[0]);

    let expected = said(&[], &[("T3", "T1")]);
    assert_eq!(shown(&served), expected);
    assert_eq!(saved(&mut served), expected);
}

/// The source file undone once (back to `T2`), the declaring file left at `T3`: the first rename's line.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames show and land, file by file, what the buffers hold however they were undone"
)]
fn undone_once_in_the_source_file() {
    let (mut served, steps) = composed();
    served.edit(SOURCE, &steps.source[1]);

    let expected = said(&[("T1", "T2")], &[("T3", "T1")]);
    assert_eq!(shown(&served), expected);
    assert_eq!(saved(&mut served), expected);
}

/// The declaring file undone once (back to `T2`), the source file left at `T3`: the first rename's move, the second's
/// line.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames show and land, file by file, what the buffers hold however they were undone"
)]
fn undone_once_in_the_declaring_file() {
    let (mut served, steps) = composed();
    served.edit(SPEC, &steps.spec[1]);

    let expected = said(&[("T1", "T3")], &[("T2", "T1")]);
    assert_eq!(shown(&served), expected);
    assert_eq!(saved(&mut served), expected);
}

/// Both files undone to `T1`, then redone to `T3`: nothing shows while undone, and redone, both show and land again.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames undone and redone show and land again"
)]
fn undone_then_redone() {
    let (mut served, steps) = composed();
    served.edit(SOURCE, &steps.source[0]);
    served.edit(SPEC, &steps.spec[0]);
    assert_eq!(shown(&served), said(&[], &[]));

    served.edit(SOURCE, &steps.source[2]);
    served.edit(SPEC, &steps.spec[2]);

    let expected = said(&[("T1", "T3")], &[("T3", "T1")]);
    assert_eq!(shown(&served), expected);
    assert_eq!(saved(&mut served), expected);
}

/// The declaring file typed to a third name by hand, the source file left at `T3`: no rename wrote that name, so no
/// move; the source's line stands.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames show and land, file by file, what the buffers hold however they were undone"
)]
fn a_third_name_typed_by_hand() {
    let (mut served, steps) = composed();
    served.edit(SPEC, &steps.spec[2].replacen("truth T3 ", "truth T9 ", 1));

    let expected = said(&[("T1", "T3")], &[]);
    assert_eq!(shown(&served), expected);
    assert_eq!(saved(&mut served), expected);
}

/// The declaring file saved and the source file not: nothing lands while a buffer holds what the disk does not. The
/// source closed unsaved, its disk text (collected) is what it says: the moves land, and no stale line.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames land once no file of theirs holds an unsaved buffer, what the disk says"
)]
fn a_source_closed_unsaved_lands_the_moves_alone() {
    let (mut served, _) = composed();
    served.write(SPEC, &buffer(&served, SPEC));
    let event = FileEvent {
        uri: served.uri(SPEC),
        typ: FileChangeType::CHANGED,
    };
    served.apply(specforge_lsp::changes::Change::Watched(vec![event]));
    assert_eq!(landed(&served), said(&[], &[]));
    assert_eq!(served.state().pending().open().len(), 2);

    served.close(SOURCE);

    assert_eq!(landed(&served), said(&[], &[("T3", "T1")]));
    assert!(served.state().pending().shown().is_empty());
}

/// The declaring file undone once (back to `T2`), then `T2` renamed to `T9`: the chain goes on from the first rename,
/// and the second's text, which the source file still holds, keeps its line.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames show and land, file by file, what the buffers hold however they were undone"
)]
fn a_rename_after_an_undo_in_one_file_keeps_what_the_other_holds() {
    let (mut served, steps) = composed();
    served.edit(SPEC, &steps.spec[1]);
    let uri = served.uri(SPEC);
    let edit = answers::rename(&served.state(), &uri, Position::new(0, 7), "T9")
        .unwrap()
        .expect("an edit");
    apply_to_buffers(&mut served, edit);
    assert!(buffer(&served, SPEC).starts_with("truth T9 "));
    assert_eq!(buffer(&served, SOURCE), steps.source[2]);

    let expected = said(&[("T1", "T3")], &[("T9", "T1")]);
    assert_eq!(shown(&served), expected);
    assert_eq!(saved(&mut served), expected);
}

/// The declaring file closed unsaved (its disk text declares `T1`), the source buffer left at `T3`: the checks read
/// the source's line alone. The source closed unsaved too, nothing lands, and the renames are over.
#[spec(
    behavior = "land_rename_ledger",
    verify = "composed editor renames land once no file of theirs holds an unsaved buffer, what the disk says"
)]
fn the_declaring_file_closed_unsaved_shows_what_the_source_holds() {
    let (mut served, _) = composed();
    served.close(SPEC);

    assert_eq!(shown(&served), said(&[("T1", "T3")], &[]));
    assert_eq!(served.state().pending().open().len(), 2);

    served.close(SOURCE);

    assert_eq!(landed(&served), said(&[], &[]));
    assert!(served.state().pending().open().is_empty());
}
