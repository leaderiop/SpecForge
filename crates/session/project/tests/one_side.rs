//! A relation is declared from one side (ADR 0150): the builtins of each set together declare no
//! relation from both ends. A set is built into one project at a time (ADR 0369 D3).

use serde_json::json;
use specforge_component::ComponentRuntime;
use specforge_component::builtins::Family;
use specforge_project::Environment;
use tempfile::TempDir;

#[specforge_test_macros::test(requirement = "FEATURE-018")]
fn no_builtin_declares_a_relation_from_both_sides() {
    for family in Family::ALL {
        let dir = TempDir::new().unwrap();
        let all: Vec<&str> = family.builtins().map(|builtin| builtin.name).collect();
        std::fs::write(
            dir.path().join("specforge.json"),
            json!({ "name": "p", "version": "0.1.0", "extensions": all }).to_string(),
        )
        .unwrap();
        let env = Environment::load(
            &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
            Some(std::sync::Arc::new(ComponentRuntime::new())),
        );
        let registries = env.registries();
        let both: Vec<_> = env.diagnostics().filter(|d| d.code == "W162").collect();
        assert!(both.is_empty(), "{family:?}: {both:?}");
        assert!(
            !registries.rules().iter().any(|rule| rule.code() == "E051"),
            "{family:?}: E051 validates `triggers`, a field no extension declares"
        );
    }
}
