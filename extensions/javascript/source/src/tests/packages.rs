//! The package facts and the package a file's items name.

use super::*;

#[test]
fn two_packages_of_one_name_merge_their_items_into_the_shallowest() {
    let facts = run(
        host(&[
            APP,
            ("examples/app/package.json", r#"{"name":"app"}"#),
            ("src/a.js", "export const x = 1;\n"),
            ("examples/app/src/a.js", "export const x = 2;\n"),
        ]),
        &[],
    )
    .0
    .unwrap();
    assert_eq!(items(&facts), ["js:app/src/a.js::x"]);
    let x = item(&facts, "js:app/src/a.js::x");
    let files: Vec<&str> = places(x).iter().map(|p| p.0).collect();
    assert_eq!(files, ["examples/app/src/a.js", "src/a.js"]);
    let packages: Vec<&Fact> = facts.iter().filter(|f| f.kind == "package").collect();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].id, "pkg:npm/app");
    assert_eq!(packages[0].fields["path"], "");
}
