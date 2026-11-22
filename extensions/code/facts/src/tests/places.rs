//! Where a report's location lies among the served item and package facts.

use crate::*;
use specforge_extension_sdk::prelude::{Fact, FactLocation};

/// `file` from line `start` (column 1) to line `end`, column `end_col` (end exclusive).
fn span(file: &str, start: usize, end: usize, end_col: usize) -> FactLocation {
    FactLocation {
        file: file.to_string(),
        start_line: start,
        start_col: 1,
        end_line: end,
        end_col,
    }
}

fn item(id: &str, place: FactLocation) -> Fact {
    Fact::new("item", id).at(place)
}

fn package(id: &str, path: &str) -> Fact {
    Fact::new("package", id).field("path", path)
}

/// A Rust file with `mod tests` at lines 10-31 holding `fn adds` at 12-16, a method of an impl at 40-45, an item
/// whose place ends at column 1 of line 16, and two one-line items on line 50; the module is served as a collector
/// receives it, parsed from JSON.
fn served() -> Vec<Fact> {
    let module: Fact = serde_json::from_str(
        r#"{"kind": "item", "id": "rust:demo::tests", "fields": {"kind": "mod"},
            "at": [{"file": "src/lib.rs", "start_line": 10, "start_col": 1, "end_line": 31, "end_col": 2}]}"#,
    )
    .expect("a served item");
    vec![
        module,
        item("rust:demo::tests::adds", span("src/lib.rs", 12, 16, 6)),
        item(
            "rust:demo::<Board as Display>::fmt",
            span("src/lib.rs", 40, 45, 6),
        ),
        item("rust:demo::ends_at_col_one", span("src/main.rs", 12, 16, 1)),
        item("rust:demo::outer", span("src/main.rs", 1, 30, 2)),
        item("rust:demo::b", span("src/lib.rs", 50, 50, 20)),
        item("rust:demo::a", span("src/lib.rs", 50, 50, 30)),
    ]
}

#[test]
fn a_line_resolves_to_the_innermost_item_holding_it() {
    let served = served();
    let places = Places::new(&served);
    assert_eq!(
        places.innermost("src/lib.rs", 13),
        Some("rust:demo::tests::adds")
    );
    assert_eq!(places.innermost("src/lib.rs", 20), Some("rust:demo::tests"));
    assert_eq!(
        places.innermost("src/lib.rs", 41),
        Some("rust:demo::<Board as Display>::fmt")
    );
    assert_eq!(places.innermost("src/lib.rs", 46), None);
}

#[test]
fn a_place_ending_at_column_one_does_not_hold_its_end_line() {
    let served = served();
    let places = Places::new(&served);
    assert_eq!(
        places.innermost("src/main.rs", 15),
        Some("rust:demo::ends_at_col_one")
    );
    assert_eq!(
        places.innermost("src/main.rs", 16),
        Some("rust:demo::outer")
    );
}

#[test]
fn two_items_on_one_line_resolve_to_the_smaller_id() {
    let served = served();
    let places = Places::new(&served);
    assert_eq!(places.innermost("src/lib.rs", 50), Some("rust:demo::a"));
}

#[test]
fn an_unserved_file_has_no_item() {
    let served = served();
    let places = Places::new(&served);
    assert_eq!(places.innermost("src/other.rs", 13), None);
    assert_eq!(Places::new(&[]).innermost("src/lib.rs", 13), None);
}

#[test]
fn a_file_belongs_to_the_package_of_the_longest_directory_holding_it() {
    let served = vec![
        package("pkg:cargo/root", ""),
        package("pkg:cargo/a", "crates/a"),
        Fact::new("package", "pkg:npm/no-path"),
    ];
    let places = Places::new(&served);
    assert_eq!(places.package_of("crates/a/src/x.rs"), Some("pkg:cargo/a"));
    // `crates/a` does not hold `crates/ab`; of the two root packages (no `path` reads `""`) the smaller id wins.
    assert_eq!(places.package_of("crates/ab/x.rs"), Some("pkg:cargo/root"));
    assert_eq!(places.package_of("crates/a"), Some("pkg:cargo/root"));
    assert_eq!(Places::new(&[]).package_of("x.rs"), None);
}

#[test]
fn a_directory_finds_its_packages_of_one_type_else_those_under_it() {
    let served = vec![
        package("pkg:npm/web", ""),
        package("pkg:cargo/b", "crates/b"),
        package("pkg:cargo/a", "crates/a"),
        package("pkg:cargo/a2", "crates/a/inner"),
        package("pkg:cargo/x", "other/x"),
    ];
    let places = Places::new(&served);
    assert_eq!(places.packages_of_dir("crates/a", "cargo"), ["pkg:cargo/a"]);
    assert_eq!(
        places.packages_of_dir("crates", "cargo"),
        ["pkg:cargo/a", "pkg:cargo/a2", "pkg:cargo/b"]
    );
    assert_eq!(
        places.packages_of_dir("", "cargo"),
        ["pkg:cargo/a", "pkg:cargo/a2", "pkg:cargo/b", "pkg:cargo/x"]
    );
    assert_eq!(places.packages_of_dir("", "npm"), ["pkg:npm/web"]);
    assert!(places.packages_of_dir("crate", "cargo").is_empty());
}

#[test]
fn a_package_name_finds_its_second_packages_too() {
    let served = vec![
        package("pkg:cargo/x#crates/y", "crates/y"),
        package("pkg:cargo/x", "x"),
        package("pkg:cargo/xy", "xy"),
    ];
    let places = Places::new(&served);
    assert_eq!(
        places.packages_named("pkg:cargo/x"),
        ["pkg:cargo/x", "pkg:cargo/x#crates/y"]
    );
    assert!(places.packages_named("pkg:cargo/z").is_empty());
}
