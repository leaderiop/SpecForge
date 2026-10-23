//! A source is keyed from the project root, so one file has one spelling (ADR 0242).

use specforge_project::testing::TestProject;
use specforge_test::prelude::*;

fn behavior(id: &str, extra: &str) -> String {
    format!(
        "behavior {id} \"{id}\" {{\n  category command\n  contract \"The system MUST {id}\"\n{extra}}}\n"
    )
}

#[specforge_test(
    behavior = "classify_project_changes",
    verify = "a discovered .spec file is a source keyed from the project root"
)]
#[ignore = "flipped by T7"]
fn a_source_is_keyed_from_the_project_root() {
    let a = format!(
        "use \"sub/b\"\n\n{}",
        behavior("alpha", "  invariants [nowhere]\n")
    );
    let dir = TestProject::new()
        .config(|c| c["spec_root"] = "spec".into())
        .files(&[
            ("spec/a.spec", &a),
            ("spec/sub/b.spec", &behavior("beta", "")),
        ]);
    let session = dir.session_bare();
    let project = session.project();
    assert_eq!(
        project.source_keys().into_iter().collect::<Vec<_>>(),
        ["spec/a.spec", "spec/sub/b.spec"]
    );
    let diagnostics = project.diagnostics();
    let e003: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(e003.len(), 1, "{diagnostics:?}");
    assert_eq!(e003[0].span.as_ref().unwrap().file.as_str(), "spec/a.spec");
    assert!(
        diagnostics.iter().all(|d| d.code != "E025"),
        "the import still resolves"
    );
    let root = dir.root().to_path_buf();
    assert_eq!(
        session
            .project()
            .environment()
            .source_key(&root.join("spec/sub/b.spec")),
        "spec/sub/b.spec"
    );
    dir.write("spec/gone.spec", "");
    let changes = session.changes([root.join("spec/gone.spec").as_path()]);
    assert_eq!(changes.sources(), ["spec/gone.spec"]);
}
