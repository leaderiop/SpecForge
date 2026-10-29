//! Resolving an id over the graph a writing collector is handed, as the host's `EntitySnapshot::find` does: a path, a
//! rooted path (`/D-01`), or a unique suffix of one on a `/` boundary. The host's `collect` keeps only full paths (a
//! short form is W115), so a collector that is given a short form resolves it before it answers.

use specforge_extension_sdk::prelude::*;

/// What an id names over a graph.
#[derive(Debug, PartialEq)]
pub enum Found<'a> {
    One(&'a GraphNode),
    Ambiguous,
    None,
}

pub fn find<'a>(nodes: &'a [GraphNode], key: &str) -> Found<'a> {
    let exact = key.strip_prefix('/').unwrap_or(key);
    if let Some(node) = nodes.iter().find(|n| n.id == exact) {
        return Found::One(node);
    }
    if key.starts_with('/') || key.contains(':') {
        return Found::None;
    }
    let tail = format!("/{key}");
    let mut matches = nodes.iter().filter(|n| n.id.ends_with(&tail));
    match (matches.next(), matches.next()) {
        (None, _) => Found::None,
        (Some(one), None) => Found::One(one),
        _ => Found::Ambiguous,
    }
}

/// The node `key` names, when it is exactly one of `kind`.
pub fn find_kind<'a>(nodes: &'a [GraphNode], key: &str, kind: &str) -> Option<&'a GraphNode> {
    match find(nodes, key) {
        Found::One(node) if node.kind == kind => Some(node),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: &str) -> GraphNode {
        GraphNode {
            id: id.into(),
            kind: kind.into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_short_form_resolves_like_graph_find() {
        let nodes = [
            node("v1.0/01/01-01", "plan"),
            node("v2.0/01/01-01", "plan"),
            node("v1.0/01/01-02", "plan"),
            node("D-01", "decision"),
            node("v1.0/01/D-01", "decision"),
            node("v1.0/01", "phase"),
        ];
        let one = |key: &str| match find(&nodes, key) {
            Found::One(n) => n.id.clone(),
            other => format!("{other:?}"),
        };
        assert_eq!(one("v1.0/01/01-02"), "v1.0/01/01-02", "a path");
        assert_eq!(one("01-02"), "v1.0/01/01-02", "a unique suffix");
        assert_eq!(one("/D-01"), "D-01", "a rooted path is exact");
        assert_eq!(one("D-01"), "D-01", "an exact path wins over a suffix");
        assert_eq!(find(&nodes, "01-01"), Found::Ambiguous);
        assert_eq!(
            find(&nodes, "-02"),
            Found::None,
            "a suffix ends on a boundary"
        );
        assert_eq!(
            find(&nodes, "/01-02"),
            Found::None,
            "a rooted path is not a suffix"
        );
        assert_eq!(find(&nodes, "x:y"), Found::None);
        assert_eq!(find_kind(&nodes, "01", "plan"), None, "01 names the phase");
        assert!(find_kind(&nodes, "01-02", "plan").is_some());
    }
}
