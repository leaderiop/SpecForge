//! What a scope holds at any depth (ADR 0291, behavior `gate_a_command`): the entities a gate scoped to an entity
//! looks at.

use specforge_common::Sym;
use specforge_graph::Scopes;
use specforge_test_macros::test as specforge_test;

fn placed(entries: &[(&str, &str, Option<&str>)]) -> Scopes {
    let mut scopes = Scopes::new();
    for (path, kind, scope) in entries {
        scopes.insert(Sym::new(path), Sym::new(kind), scope.map(Sym::new));
    }
    scopes
}

fn paths(found: Vec<Sym>) -> Vec<String> {
    found.iter().map(|p| p.to_string()).collect()
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a gate is scoped to its subject"
)]
fn within_names_the_members_of_a_scope_at_any_depth() {
    let scopes = placed(&[
        ("v1.0", "milestone", None),
        ("v1.0/03", "phase", Some("v1.0")),
        ("v1.0/03/03-01", "plan", Some("v1.0/03")),
        ("v1.0/03/03-02", "plan", Some("v1.0/03")),
        ("v1.0/05", "phase", Some("v1.0")),
        ("v1.0/05/05-01", "plan", Some("v1.0/05")),
        ("loose", "note", None),
    ]);
    assert_eq!(
        paths(scopes.within("v1.0/03")),
        ["v1.0/03/03-01", "v1.0/03/03-02"]
    );
    assert_eq!(
        paths(scopes.within("v1.0")),
        [
            "v1.0/03",
            "v1.0/03/03-01",
            "v1.0/03/03-02",
            "v1.0/05",
            "v1.0/05/05-01"
        ],
        "the grandchildren are within the milestone too"
    );
    assert!(
        scopes.within("v1.0/05/05-01").is_empty(),
        "a leaf scopes nothing"
    );
    assert!(scopes.within("loose").is_empty());
    assert!(scopes.within("nowhere").is_empty(), "an entity not placed");
}
