//! Where `init` scaffolds: the spec root a request names.

use std::path::Path;

use tempfile::TempDir;

use crate::writes::candidates;

/// A request names where the project reads its sources: `specforge.json` says it and the starter is written there;
/// none writes no `spec_root` (the root's own sources) and the starter at the root; a spec root that leaves the
/// directory, or is absolute, is refused before anything is written.
#[test]
fn init_writes_the_spec_root_it_is_asked_for() {
    use specforge_ops::authoring::init;
    let scratch = TempDir::new().unwrap();
    let init_at = |dir: &Path, spec_root: Option<&str>| {
        init::init(
            &init::Request {
                dir,
                name: Some("demo"),
                version: init::DEFAULT_VERSION,
                spec_root,
                extensions: &[],
                caller: specforge_ops::testing::caller(),
            },
            &candidates(),
        )
    };
    let config = |dir: &Path| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(dir.join("specforge.json")).unwrap()).unwrap()
    };

    let nested = scratch.path().join("nested");
    let outcome = init_at(&nested, Some("specs/main")).unwrap();
    assert_eq!(outcome.starter, "specs/main/hello.spec");
    assert_eq!(outcome.document().starter_file, "specs/main/hello.spec");
    assert_eq!(config(&nested)["spec_root"], "specs/main");
    assert!(nested.join("specs/main/hello.spec").is_file());
    assert!(!nested.join("spec").exists());

    let rooted = scratch.path().join("rooted");
    let outcome = init_at(&rooted, None).unwrap();
    assert_eq!(outcome.starter, "hello.spec");
    assert_eq!(config(&rooted).get("spec_root"), None);
    assert!(rooted.join("hello.spec").is_file());

    for (spec_root, why) in [
        ("../outside", "it leaves the project's directory"),
        ("/abs", "it is absolute"),
    ] {
        let dir = scratch.path().join("refused");
        let refusal = init_at(&dir, Some(spec_root)).unwrap_err();
        assert_eq!(refusal.error.code, init::INVALID_SPEC_ROOT, "{spec_root}");
        assert_eq!(refusal.subject, Some(init::InitField::SpecRoot));
        assert!(refusal.error.message.contains(why), "{refusal:?}");
        assert!(!dir.exists(), "{spec_root}: nothing written");
    }
}
