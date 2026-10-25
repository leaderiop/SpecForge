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

/// `real/{task,b}.spec` in a directory outside the project, `link` to `real` beside it, and a project whose spec root
/// is that link. Returns the project, the outside directory (canonical) and the link.
#[cfg(unix)]
fn project_over_a_linked_spec_root() -> (TestProject, tempfile::TempDir, std::path::PathBuf) {
    let outside = tempfile::TempDir::new().unwrap();
    let real = std::fs::canonicalize(outside.path()).unwrap().join("real");
    std::fs::create_dir(&real).unwrap();
    std::fs::write(real.join("task.spec"), behavior("alpha", "")).unwrap();
    std::fs::write(
        real.join("b.spec"),
        format!("use \"task\"\n\n{}", behavior("beta", "")),
    )
    .unwrap();
    let link = real.with_file_name("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let project = TestProject::new().config(|c| c["spec_root"] = link.to_str().unwrap().into());
    (project, outside, link)
}

#[cfg(unix)]
#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "an import under a spec root outside the project, through a link too, names its source"
)]
#[ignore = "flipped by T2"]
fn an_import_under_a_linked_spec_root_outside_the_project_resolves() {
    let (project, outside, _link) = project_over_a_linked_spec_root();
    let session = project.session_bare();
    let project = session.project();
    let real = std::fs::canonicalize(outside.path()).unwrap().join("real");
    let keys: Vec<_> = project.source_keys().into_iter().collect();
    assert_eq!(
        keys,
        [
            real.join("b.spec").to_str().unwrap(),
            real.join("task.spec").to_str().unwrap()
        ]
    );
    let diagnostics = project.diagnostics();
    assert!(
        diagnostics.iter().all(|d| d.code != "E025"),
        "{diagnostics:?}"
    );
}

#[cfg(unix)]
#[specforge_test(
    behavior = "classify_project_changes",
    verify = "a change under a spec root outside the project, through a link too, is that source's change"
)]
#[ignore = "flipped by T2"]
fn a_change_under_a_linked_spec_root_outside_the_project_is_its_source() {
    let (project, outside, link) = project_over_a_linked_spec_root();
    let mut session = project.session_bare();
    let real = std::fs::canonicalize(outside.path()).unwrap().join("real");
    std::fs::write(link.join("c.spec"), behavior("gamma", "")).unwrap();
    let new = real.join("c.spec").to_str().unwrap().to_owned();
    assert_eq!(
        session.changes([link.join("c.spec").as_path()]).sources(),
        [new]
    );
    let b = real.join("b.spec").to_str().unwrap().to_owned();
    let text = std::fs::read_to_string(link.join("b.spec")).unwrap();
    std::fs::write(link.join("b.spec"), format!("{text}\n// edit\n")).unwrap();
    specforge_project::testing::disk_changed(&mut session, &[b.as_str()]);
    assert!(session.project().source_keys().contains(b.as_str()));
}
