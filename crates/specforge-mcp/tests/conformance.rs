//! C9-11: the declarative surface must be compiled against the
//! implementation, not just for it. Every tool advertised in the registry
//! must have a dispatch arm in the tool-call router, and every arm must be
//! advertised — a hand-bound string drift fails here instead of at runtime.

use std::path::Path;

fn crate_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Tool names advertised by the registry (single source of truth for the
/// tool surface).
fn registry_tool_names() -> Vec<String> {
    let source = crate_file("registry.rs");
    let mut names: Vec<String> = Vec::new();
    let mut rest = source.as_str();
    while let Some(idx) = rest.find("name: \"specforge.") {
        let tail = &rest[idx + "name: \"".len()..];
        let name: String = tail.chars().take_while(|c| *c != '"').collect();
        names.push(name);
        rest = tail;
    }
    names.sort();
    names.dedup();
    names
}

/// Tool names with a dispatch arm in the tool-call handler.
fn dispatched_tool_names() -> Vec<String> {
    let source = crate_file("tools/mod.rs");
    let mut names: Vec<String> = Vec::new();
    let mut rest = source.as_str();
    while let Some(idx) = rest.find("specforge.") {
        let tail = &rest[idx + "specforge.".len()..];
        let name: String = tail
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.push(format!("specforge.{name}"));
        }
        rest = tail;
    }
    names.sort();
    names.dedup();
    names
}

#[test]
fn registry_and_dispatch_agree_on_the_tool_surface() {
    let advertised = registry_tool_names();
    assert!(
        advertised.len() >= 15,
        "registry should advertise the standard tool surface (found {})",
        advertised.len()
    );

    let dispatched = dispatched_tool_names();
    let unadvertised: Vec<&String> = dispatched
        .iter()
        .filter(|n| !advertised.contains(n))
        .collect();
    let undispatched: Vec<&String> = advertised
        .iter()
        .filter(|n| !dispatched.contains(n))
        .collect();

    assert!(
        unadvertised.is_empty(),
        "dispatch arms without a registry entry (C9-11 drift): {unadvertised:?}"
    );
    assert!(
        undispatched.is_empty(),
        "advertised tools with no dispatch arm (C9-11 drift): {undispatched:?}"
    );
}
