//! The package facts and the package a file's items name.

use super::*;

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-056")]
fn a_second_package_of_one_name_keeps_its_items_and_its_package_fact_apart() {
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
    assert_eq!(
        items(&facts),
        ["js:app/src/a.js::x", "js:app@examples%2Fapp/src/a.js::x"]
    );
    let deeper = item(&facts, "js:app@examples%2Fapp/src/a.js::x");
    assert_eq!(places(deeper)[0].0, "examples/app/src/a.js");
    assert_eq!(deeper.fields["package"], "pkg:npm/app#examples/app");
    assert_eq!(
        item(&facts, "js:app/src/a.js::x").fields["package"],
        "pkg:npm/app"
    );
    let packages: Vec<(&str, &Value)> = facts
        .iter()
        .filter(|f| f.kind == "package")
        .map(|f| (f.id.as_str(), &f.fields["path"]))
        .collect();
    assert_eq!(
        packages,
        [
            ("pkg:npm/app", &json!("")),
            ("pkg:npm/app#examples/app", &json!("examples/app"))
        ]
    );
}
