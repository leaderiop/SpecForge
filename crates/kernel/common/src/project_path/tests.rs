use super::*;

#[test]
fn a_file_under_the_root_is_relative_with_slashes() {
    let path = ProjectPath::of(Path::new("/p"), Path::new("/p/spec/a.spec"));
    assert_eq!(path.as_str(), "spec/a.spec");
    assert_eq!(path.to_string(), "spec/a.spec");
    assert_eq!(
        ProjectPath::of(Path::new("/p"), Path::new("/p/specforge.json")).as_str(),
        "specforge.json"
    );
}

#[test]
fn it_serializes_as_a_string_and_joins_back_onto_the_root() {
    let path = ProjectPath::of(Path::new("/p"), Path::new("/p/spec/a.spec"));
    assert_eq!(serde_json::to_value(&path).unwrap(), "spec/a.spec");
    assert_eq!(path.under(Path::new("/p")), Path::new("/p/spec/a.spec"));
    let outside = ProjectPath("/elsewhere/x.wasm".to_string());
    assert_eq!(
        outside.under(Path::new("/p")),
        Path::new("/elsewhere/x.wasm")
    );
}

#[cfg(unix)]
#[test]
fn a_root_spelled_through_a_link_names_the_same_file() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    std::fs::create_dir_all(real.join("spec")).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();

    // Written through the link, named from the real root; a removed file (no longer on disk) included.
    assert_eq!(
        ProjectPath::of(&real, &link.join("spec/a.spec")).as_str(),
        "spec/a.spec"
    );
    assert_eq!(
        ProjectPath::of(&real, &link.join("gone.wasm")).as_str(),
        "gone.wasm"
    );
    assert_eq!(
        ProjectPath::of(&link, &real.join("spec/a.spec")).as_str(),
        "spec/a.spec"
    );
}

#[test]
fn a_removed_file_is_named_through_its_nearest_existing_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("spec")).unwrap();
    let removed = dir.path().join("spec/deep/gone/x.spec");
    assert_eq!(
        ProjectPath::of(dir.path(), &removed).as_str(),
        "spec/deep/gone/x.spec"
    );
}

#[test]
fn a_file_outside_the_root_is_absolute_and_canonical() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();

    let named = root.join("../elsewhere/x.wasm");
    let expected = std::fs::canonicalize(&elsewhere).unwrap().join("x.wasm");
    assert_eq!(
        ProjectPath::of(&root, &named).as_str(),
        expected.display().to_string()
    );
    assert_eq!(
        ProjectPath::absolute(&named).as_str(),
        expected.display().to_string()
    );
}

#[test]
fn a_name_through_a_missing_directory_and_back_is_named_from_the_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("out")).unwrap();

    assert_eq!(
        ProjectPath::of(dir.path(), &dir.path().join("nope/../out/brief.json")).as_str(),
        "out/brief.json"
    );
    let expected = std::fs::canonicalize(dir.path())
        .unwrap()
        .join("elsewhere/x");
    assert_eq!(
        ProjectPath::absolute(&dir.path().join("nope/../elsewhere/x")).as_str(),
        expected.display().to_string()
    );
}

#[cfg(unix)]
#[test]
fn a_link_inside_a_differently_spelled_root_keeps_its_own_name() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    std::fs::create_dir_all(real.join("shared")).unwrap();
    std::fs::write(real.join("shared/real.spec"), "").unwrap();
    std::os::unix::fs::symlink("shared/real.spec", real.join("alias.spec")).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();

    // The root through `link`, the file under `real`: the plain forms differ, the link is not followed.
    for named in [
        ProjectPath::of(&link, &real.join("alias.spec")),
        ProjectPath::key(&link, &real.join("alias.spec")),
    ] {
        assert_eq!(named.as_str(), "alias.spec");
    }
}

#[test]
fn a_source_key_is_its_project_path() {
    let key = ProjectPath::key(Path::new("/p"), Path::new("/p/spec/a.spec"));
    assert_eq!(key.as_str(), "spec/a.spec");
    assert_eq!(AsRef::<str>::as_ref(&key), "spec/a.spec");
    assert_eq!(
        key,
        ProjectPath::of(Path::new("/p"), Path::new("/p/spec/a.spec"))
    );
}

#[test]
fn a_key_outside_the_root_is_absolute_with_its_own_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();
    let expected = std::fs::canonicalize(&elsewhere).unwrap().join("x.spec");
    assert_eq!(
        ProjectPath::key(&root, &elsewhere.join("x.spec")).as_str(),
        expected.display().to_string()
    );
    // No project: every absolute path is its own key.
    assert_eq!(
        ProjectPath::key(Path::new(""), &elsewhere.join("x.spec")).as_str(),
        expected.display().to_string()
    );
}

#[test]
fn a_name_that_is_no_path_is_its_own_key() {
    assert_eq!(
        ProjectPath::key(Path::new("/p"), Path::new("untitled:Untitled-1")).as_str(),
        "untitled:Untitled-1"
    );
    assert_eq!(
        ProjectPath::key(Path::new(""), Path::new("untitled:Untitled-1")).as_str(),
        "untitled:Untitled-1"
    );
}
