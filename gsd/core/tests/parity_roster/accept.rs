//! The numbers of `gsd/parity/ACCEPTANCE.md` (D-r44-17): for each family, how many of its items `@gsd/core` builds, an
//! extension owns, do not apply, or are named and not built. Computed from the tables the roster check already holds;
//! `the_acceptance_counts_match` compares them with what the file says.

use std::collections::BTreeSet;

use serde_json::Value;

use super::cutover::{Class, GATE_CUTOVER};
use super::evidence::{Evidence, RULE_EVIDENCE};
use super::{
    AGENTS, ASSET_FAMILIES, CAPABILITIES, CAPABILITY_IDS, CONFIG_SCHEMA, HOOK_ROSTER, MODULES,
    Module, NOT_EMBEDDED, Owner, README, ROSTER, Why, asset_path, embedded, family, module_rows,
};

const SETTINGS_MD: &str = include_str!("../../spec/settings.md");

/// `[total, built, owned, not applicable, named and not built]`.
pub type Counts = [usize; 5];

fn counts(built: usize, owned: usize, na: usize, open: usize) -> Counts {
    [built + owned + na + open, built, owned, na, open]
}

pub fn commands() -> Counts {
    let built = ROSTER.iter().filter(|(_, o)| o.is_core()).count();
    let owned = ROSTER
        .iter()
        .filter(|(_, o)| matches!(o, Owner::Extension(_)))
        .count();
    counts(built, owned, 0, 0)
}

pub fn agents() -> Counts {
    let all = family("agents");
    let built = all
        .iter()
        .filter(|a| {
            let base = a.strip_suffix(".compact").unwrap_or(a);
            AGENTS.iter().any(|x| x.id == base)
        })
        .count();
    counts(built, all.len() - built, 0, 0)
}

pub fn hooks() -> Counts {
    use gsd_core::hooks::roster::Disposition;
    let of = |f: fn(&Disposition) -> bool| HOOK_ROSTER.iter().filter(|(_, d)| f(d)).count();
    // Windsurf is a later host: its renderings are named and open, not built.
    let open = of(|d| matches!(d, Disposition::Rendering("windsurf")));
    let na = of(|d| matches!(d, Disposition::Dropped(_)));
    counts(HOOK_ROSTER.len() - na - open, 0, na, open)
}

pub fn capabilities() -> Counts {
    let table = gsd_kit::markdown::tables(README)
        .into_iter()
        .find(|t| t.heading == "GSD capabilities, one by one")
        .expect("the capabilities table");
    let (mut built, mut owned, mut open) = (0, 0, 0);
    for cells in &table.rows {
        let n = cells[0].split(',').count();
        let lands = cells[2].as_str();
        // The `specforged` harness is not built: its two capabilities have their settings keys and nothing else.
        if lands.starts_with("`@gsd/core` harness") || lands.starts_with("install renderers, later")
        {
            open += n;
        } else if lands.starts_with("`@gsd/core`") || lands.starts_with("install renderers, first")
        {
            built += n;
        } else {
            assert!(lands.starts_with("`@gsd/"), "{lands}");
            owned += n;
        }
    }
    let ids: Vec<String> = serde_json::from_str(CAPABILITY_IDS).expect("capability ids");
    assert_eq!(built + owned + open, ids.len());
    counts(built, owned, 0, open)
}

pub fn assets() -> Counts {
    let (mut built, mut owned, mut na) = (0, 0, 0);
    for fam in ASSET_FAMILIES {
        for item in family(fam) {
            if embedded().contains(asset_path(fam, &item).as_str()) {
                built += 1;
                continue;
            }
            let why = NOT_EMBEDDED
                .iter()
                .find(|(f, i, _)| *f == fam && *i == item)
                .map(|(_, _, why)| why)
                .expect("a NOT_EMBEDDED row");
            match why {
                Why::Command(_) => built += 1,
                Why::Owned(_) => owned += 1,
                Why::NotApplicable(_) => na += 1,
            }
        }
    }
    counts(built, owned, na, 0)
}

pub fn cli_modules() -> Counts {
    let (mut built, mut owned, mut na) = (0, 0, 0);
    for module in family("cli_modules") {
        let pattern = module_rows(&module)[0];
        match MODULES
            .iter()
            .find(|(p, _)| *p == pattern)
            .expect("a row")
            .1
        {
            Module::Ported(_) | Module::Host(_) => built += 1,
            Module::Owned(_) => owned += 1,
            Module::NotApplicable(_) => na += 1,
        }
    }
    counts(built, owned, na, 0)
}

/// The code spans of a table cell.
fn backticked(cell: &str) -> Vec<String> {
    gsd_kit::markdown::inline_code(cell)
        .into_iter()
        .map(|(_, code)| code)
        .collect()
}

fn names(token: &str, key: &str) -> bool {
    match token.strip_suffix('*') {
        Some(prefix) => key.starts_with(prefix),
        None => token == key,
    }
}

/// The 212 keys of the roster: the manifest's and the capabilities', split as `spec/settings.md` splits them.
pub fn config_keys() -> Counts {
    let schema: Value = serde_json::from_str(CONFIG_SCHEMA).expect("schema");
    let capabilities: Value = serde_json::from_str(CAPABILITIES).expect("capabilities");
    let mut keys: BTreeSet<&str> = schema["validKeys"]
        .as_array()
        .expect("validKeys")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    keys.extend(
        capabilities
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str),
    );
    let settings = gsd_kit::markdown::tables(SETTINGS_MD);
    let table = |pick: &dyn Fn(&gsd_kit::markdown::Table) -> bool| {
        settings
            .iter()
            .find(|t| pick(t))
            .expect("a settings.md table")
    };
    let owners: Vec<Vec<String>> = table(&|t| t.header[0] == "Owner")
        .rows
        .iter()
        .filter(|cells| cells[0].starts_with("`@gsd/") && cells[0] != "`@gsd/core`")
        .map(|cells| backticked(&cells[1]))
        .collect();
    let na: Vec<String> = table(&|t| t.heading == "Not applicable")
        .rows
        .iter()
        .map(|cells| cells[0].as_str())
        .filter(|cell| !cell.contains("API key half"))
        .flat_map(backticked)
        .collect();
    let owned = keys
        .iter()
        .filter(|k| {
            owners
                .iter()
                .any(|tokens| tokens.iter().any(|t| names(t, k)))
        })
        .count();
    let not_applicable = keys
        .iter()
        .filter(|k| na.iter().any(|t| names(t, k)))
        .count();
    counts(
        keys.len() - owned - not_applicable,
        owned,
        not_applicable,
        0,
    )
}

pub fn gate_cutover() -> Counts {
    let of = |f: fn(&Class) -> bool| GATE_CUTOVER.iter().filter(|(_, c)| f(c)).count();
    counts(
        of(|c| matches!(c, Class::Reproduced(_) | Class::ByConstruction(_))),
        of(|c| matches!(c, Class::Owned(_))),
        of(|c| matches!(c, Class::NotApplicable(_))),
        of(|c| matches!(c, Class::NotReproduced(_) | Class::Diverges(_))),
    )
}

pub fn codes() -> Counts {
    let built = RULE_EVIDENCE
        .iter()
        .filter(|(_, e)| matches!(e, Evidence::Tests { fires, silent } if !fires.is_empty() && !silent.is_empty()))
        .count();
    counts(built, 0, 0, 0)
}

/// The row of `family` in the checklist table of the file, as numbers.
pub fn row(file: &str, family: &str) -> Counts {
    let table = gsd_kit::markdown::tables(file)
        .into_iter()
        .find(|t| t.header[..2] == ["Family", "Total"])
        .expect("the family table");
    let line = table
        .rows
        .iter()
        .find(|cells| cells[0] == family)
        .unwrap_or_else(|| panic!("no row for {family}"));
    let cells: Vec<usize> = line
        .iter()
        .skip(1)
        .take(5)
        .map(|c| {
            c.trim()
                .parse()
                .unwrap_or_else(|_| panic!("{family}: `{c}` is no count"))
        })
        .collect();
    cells.try_into().expect("five counts")
}
