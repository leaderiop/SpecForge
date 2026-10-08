//! How the registries a command asks are read out of `specforge.json`: the
//! way `add`, `update` and `remove` read it, so a project whose config is
//! unusable is refused the same on every command.

use specforge_ops_registry::configured;
use specforge_test_macros::test as specforge_test;
use tempfile::TempDir;

fn project(config: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("specforge.json"), config).unwrap();
    dir
}

#[specforge_test(
    behavior = "search_registry",
    verify = "an unusable specforge.json is refused with the refusal add gives, before any network call"
)]
fn an_unusable_specforge_json_is_refused_as_add_refuses_it() {
    for config in ["{ \"extensions\": [", "[1, 2]", "{\"extensions\": 7}"] {
        let dir = project(config);
        let refused = specforge_ops::config::required(dir.path()).unwrap_err();

        for operation in ["search", "login", "publish", "add"] {
            let error = configured(dir.path(), operation).unwrap_err();

            assert_eq!(error, refused, "{config}: {operation}");
            assert_eq!(error.code, "config_invalid");
        }
    }
}

#[test]
fn a_usable_config_without_registries_is_e063_and_an_unreadable_entry_is_e067() {
    let none = project(r#"{"name": "p", "extensions": []}"#);
    assert_eq!(configured(none.path(), "search").unwrap_err().code, "E063");

    // Not a project at all: no registry, as before.
    let nothing = TempDir::new().unwrap();
    assert_eq!(
        configured(nothing.path(), "search").unwrap_err().code,
        "E063"
    );

    let unreadable = project(r#"{"registries": [{"url": "https://r.example"}]}"#);
    assert_eq!(
        configured(unreadable.path(), "search").unwrap_err().code,
        "E067"
    );

    let good = project(
        r#"{"registries": [{"alias": "main", "url": "https://r.example", "default": true}]}"#,
    );
    let found = configured(good.path(), "search").unwrap();
    assert_eq!(found.registries.len(), 1);
    assert_eq!(found.registries[0].alias, "main");
}
