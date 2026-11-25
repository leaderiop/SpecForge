//! The `coverage` facts minted from a coverage report's files and the served items.

use crate::*;
use specforge_extension_sdk::prelude::{Fact, FactLocation};

fn item(id: &str, file: &str, start: usize, end: usize) -> Fact {
    Fact::new("item", id).at(FactLocation {
        file: file.to_string(),
        start_line: start,
        start_col: 1,
        end_line: end,
        end_col: 2,
    })
}

/// `mod tests` at lines 10-30 holding `fn adds` at 12-16, and `fn add` at 1-5.
fn served() -> Vec<Fact> {
    vec![
        item("rust:demo::add", "src/lib.rs", 1, 5),
        item("rust:demo::tests", "src/lib.rs", 10, 30),
        item("rust:demo::tests::adds", "src/lib.rs", 12, 16),
    ]
}

fn function(line: usize, name: &str, hits: u64) -> Function {
    Function {
        line,
        name: name.to_string(),
        hits,
    }
}

fn branch(line: usize, branch: &str, taken: Option<u64>) -> Branch {
    Branch {
        line,
        block: "0".to_string(),
        branch: branch.to_string(),
        taken,
    }
}

fn lib(functions: Vec<Function>, lines: Vec<(usize, u64)>, branches: Vec<Branch>) -> CoverageFile {
    CoverageFile {
        file: "src/lib.rs".to_string(),
        functions,
        lines,
        branches,
    }
}

/// The fact `id` of `facts`.
fn fact<'f>(facts: &'f [Fact], id: &str) -> &'f Fact {
    facts
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("no fact {id} in {facts:?}"))
}

/// The integer field `name` of `fact`.
fn count(fact: &Fact, name: &str) -> Option<u64> {
    fact.fields.get(name).and_then(|v| v.as_u64())
}

#[test]
fn an_item_counts_its_functions_lines_and_branches() {
    let files = [lib(
        vec![function(1, "add", 3), function(1, "add_u8", 0)],
        vec![(2, 3), (3, 0), (4, 1)],
        vec![
            branch(3, "0", Some(2)),
            branch(3, "1", Some(0)),
            branch(4, "0", None),
        ],
    )];
    let facts = coverage_facts(&files, &served());
    assert_eq!(facts.len(), 1, "{facts:?}");
    let add = fact(&facts, "cov:rust:demo::add");
    assert_eq!(add.kind, "coverage");
    assert_eq!(add.fields["item"], "rust:demo::add");
    assert_eq!(add.fields["file"], "src/lib.rs");
    let counts: Vec<Option<u64>> = [
        "functions_found",
        "functions_hit",
        "lines_found",
        "lines_hit",
        "branches_found",
        "branches_hit",
    ]
    .iter()
    .map(|name| count(add, name))
    .collect();
    assert_eq!(
        counts,
        [Some(2), Some(1), Some(3), Some(2), Some(3), Some(1)]
    );
    assert!(add.at.is_empty());
}

#[test]
fn a_line_counts_for_the_innermost_item_holding_it() {
    let files = [lib(vec![], vec![(13, 1), (20, 1), (21, 0)], vec![])];
    let facts = coverage_facts(&files, &served());
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["cov:rust:demo::tests", "cov:rust:demo::tests::adds"]);
    let adds = fact(&facts, "cov:rust:demo::tests::adds");
    assert_eq!(
        (count(adds, "lines_found"), count(adds, "lines_hit")),
        (Some(1), Some(1))
    );
    let module = fact(&facts, "cov:rust:demo::tests");
    assert_eq!(
        (count(module, "lines_found"), count(module, "lines_hit")),
        (Some(2), Some(1))
    );
}

#[test]
fn what_no_item_holds_is_kept_per_file_with_no_item() {
    let files = [
        lib(vec![function(0, "orphan", 1)], vec![(7, 1)], vec![]),
        CoverageFile {
            file: "src/bin/my tool.rs".to_string(),
            lines: vec![(1, 0)],
            ..CoverageFile::default()
        },
    ];
    let facts = coverage_facts(&files, &served());
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["cov:@src%2Fbin%2Fmy%20tool.rs", "cov:@src%2Flib.rs"]);
    let unheld = fact(&facts, "cov:@src%2Flib.rs");
    assert!(!unheld.fields.contains_key("item"));
    assert_eq!(unheld.fields["file"], "src/lib.rs");
    assert_eq!(
        (count(unheld, "functions_found"), count(unheld, "lines_hit")),
        (Some(1), Some(1))
    );
    let tool = fact(&facts, "cov:@src%2Fbin%2Fmy%20tool.rs");
    assert_eq!(tool.fields["file"], "src/bin/my tool.rs");
    assert_eq!(count(tool, "lines_found"), Some(1));
    assert_eq!(count(tool, "functions_found"), None);
}

#[test]
fn two_sections_of_one_file_merge() {
    let files = [
        lib(vec![function(1, "add", 0)], vec![(2, 0), (3, 0)], vec![]),
        lib(
            vec![function(1, "add", 2)],
            vec![(2, 1)],
            vec![branch(2, "0", None)],
        ),
        lib(vec![], vec![], vec![branch(2, "0", Some(0))]),
    ];
    let facts = coverage_facts(&files, &served());
    let add = fact(&facts, "cov:rust:demo::add");
    assert_eq!(
        (count(add, "functions_found"), count(add, "functions_hit")),
        (Some(1), Some(1))
    );
    assert_eq!(
        (count(add, "lines_found"), count(add, "lines_hit")),
        (Some(2), Some(1))
    );
    // `-` then `0`: evaluated, never taken.
    assert_eq!(
        (count(add, "branches_found"), count(add, "branches_hit")),
        (Some(1), Some(0))
    );
}

#[test]
fn a_repeated_function_counts_once() {
    let files = [lib(
        vec![
            function(1, "add", 1),
            function(1, "add", 1),
            function(2, "add", 0),
        ],
        vec![],
        vec![],
    )];
    let facts = coverage_facts(&files, &served());
    assert_eq!(facts.len(), 1);
    let add = fact(&facts, "cov:rust:demo::add");
    assert_eq!(
        (count(add, "functions_found"), count(add, "functions_hit")),
        (Some(2), Some(1))
    );
}

#[test]
fn a_file_with_no_branch_record_has_no_branch_fields() {
    let files = [lib(vec![], vec![(2, 1)], vec![])];
    let facts = coverage_facts(&files, &served());
    let add = fact(&facts, "cov:rust:demo::add");
    let names: Vec<&str> = add.fields.keys().map(String::as_str).collect();
    assert_eq!(names, ["file", "item", "lines_found", "lines_hit"]);
}

#[test]
fn a_family_the_file_holds_counts_zero_for_an_item_it_misses() {
    // The file has branches, all in `add`; `adds` gets branch counts of zero, since the report measured branches there.
    let files = [lib(
        vec![],
        vec![(2, 1), (13, 1)],
        vec![branch(2, "0", Some(1))],
    )];
    let facts = coverage_facts(&files, &served());
    let adds = fact(&facts, "cov:rust:demo::tests::adds");
    assert_eq!(
        (count(adds, "branches_found"), count(adds, "branches_hit")),
        (Some(0), Some(0))
    );
}

#[test]
fn no_record_mints_nothing() {
    assert!(coverage_facts(&[], &served()).is_empty());
    assert!(coverage_facts(&[lib(vec![], vec![], vec![])], &served()).is_empty());
}
