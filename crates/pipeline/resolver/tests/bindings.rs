//! A selective `use` binding the target does not export (ADR 0233).

use specforge_parser::parse;
use specforge_resolver::resolve_imports;
use specforge_test_macros::test as specforge_test;

/// Each W027 of `files` as `"<file> l:c-l:c <message>"`.
fn w027(files: &[(&str, &str)]) -> Vec<String> {
    let parsed: Vec<_> = files.iter().map(|(k, t)| (*k, parse(t, k))).collect();
    let refs: Vec<_> = parsed.iter().map(|(k, f)| (*k, f)).collect();
    resolve_imports(&refs, &[])
        .iter()
        .filter(|d| d.code == "W027")
        .map(|d| {
            let s = d.span.as_ref().unwrap();
            format!(
                "{} {}:{}-{}:{} {}",
                s.file, s.start_line, s.start_col, s.end_line, s.end_col, d.message
            )
        })
        .collect()
}

#[ignore = "flipped by T7"]
#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "a selective use of a name its target does not export is W027"
)]
fn a_selective_use_of_a_name_its_target_does_not_export_is_w027() {
    let found = w027(&[
        ("a.spec", "behavior f1 \"F1\" { contract \"c\" }"),
        ("barrel.spec", "pub use { f1 } from \"a\"\n"),
        (
            "c.spec",
            "use { f1 as g, Nope } from \"a\"\nuse { f1 } from \"barrel\"\n",
        ),
    ]);
    assert_eq!(
        found,
        ["c.spec 1:16-1:20 selective import 'Nope' not found in target 'a.spec'"]
    );
}
