//! A held path keeps three names, one use each (ADR 0272): `named`, `at`, `real`.

use super::*;

fn scene() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let real = std::fs::canonicalize(dir.path()).unwrap();
    std::fs::create_dir_all(real.join("proj/sub")).unwrap();
    (dir, real.join("proj"))
}

#[test]
fn a_held_name_keeps_the_callers_spelling_beside_where_it_is_read() {
    let (_dir, root) = scene();
    let held = Containment::within(&root)
        .resolve(Path::new("a/../b.json"))
        .unwrap();
    assert_eq!(held.named, Path::new("a/../b.json"));
    assert_eq!(held.at, root.join("a/../b.json"));
    assert!(held.at.is_absolute());
    assert_eq!(held.real, root.join("b.json"));

    let held = Containment::unbounded()
        .resolve(Path::new("a/../b.json"))
        .unwrap();
    assert_eq!(held.named, held.at);
}

#[test]
fn resolve_from_reads_a_relative_name_against_its_base_and_an_absolute_one_as_given() {
    let (_dir, root) = scene();
    let containment = Containment::within(&root);
    let base = root.join("sub");

    let held = containment
        .resolve_from(&base, Path::new("x.wasm"))
        .unwrap();
    assert_eq!(held.named, Path::new("x.wasm"));
    assert_eq!(held.at, base.join("x.wasm"));

    let absolute = root.join("y.wasm");
    let held = containment.resolve_from(&base, &absolute).unwrap();
    assert_eq!((held.named, held.at), (absolute.clone(), absolute));

    let outside = containment
        .resolve_from(&base, Path::new("../../out"))
        .unwrap_err();
    assert_eq!(outside.named(), Path::new("../../out"));
}

#[cfg(unix)]
#[test]
fn an_entry_of_a_held_directory_is_named_under_its_name() {
    let (_dir, root) = scene();
    let containment = Containment::within(&root);
    let dir = containment.resolve(Path::new("sub")).unwrap();

    let entry = dir.entry(&containment, Path::new("x.json")).unwrap();
    assert_eq!(entry.named, Path::new("sub/x.json"));
    assert_eq!(entry.at, root.join("sub/x.json"));

    std::os::unix::fs::symlink("/", root.join("sub/out")).unwrap();
    let outside = dir.entry(&containment, Path::new("out")).unwrap_err();
    assert_eq!(outside.named(), Path::new("sub/out"));
}

#[cfg(unix)]
#[test]
fn shown_names_a_held_file_from_the_root_a_link_by_its_own_name() {
    let (dir, root) = scene();
    std::fs::create_dir_all(root.join("ext")).unwrap();
    std::fs::write(root.join("ext/greet.wasm"), "w").unwrap();
    std::os::unix::fs::symlink("ext/greet.wasm", root.join("alias.wasm")).unwrap();
    let containment = Containment::within(&root);

    let shown = |name: &str| {
        containment
            .resolve(Path::new(name))
            .unwrap()
            .shown(&root)
            .into_string()
    };
    assert_eq!(shown("alias.wasm"), "alias.wasm");
    assert_eq!(shown("sub/../ext/greet.wasm"), "ext/greet.wasm");

    let through = dir.path().join("through");
    std::os::unix::fs::symlink(&root, &through).unwrap();
    let held = containment.resolve(Path::new("alias.wasm")).unwrap();
    assert_eq!(held.shown(&through).into_string(), "alias.wasm");
}
