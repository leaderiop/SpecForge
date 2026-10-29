use super::*;

fn node<'a>(id: &'a str, depends_on: &[&'a str], halted: bool) -> Node<'a> {
    Node {
        id,
        depends_on: depends_on.to_vec(),
        halted,
    }
}

fn ids<'a>(nodes: &[Node<'a>]) -> Vec<&'a str> {
    blocked(nodes).into_iter().collect()
}

#[test]
fn halt_propagation_is_gsds() {
    // A chain: b depends on halted a, c on b.
    let chain = [
        node("a", &[], true),
        node("b", &["a"], false),
        node("c", &["b"], false),
        node("d", &[], false),
    ];
    assert_eq!(ids(&chain), ["b", "c"]);
    // A diamond: d depends on b and c, which depend on halted a; and on the healthy e.
    let diamond = [
        node("a", &[], true),
        node("b", &["a"], false),
        node("c", &["a"], false),
        node("e", &[], false),
        node("d", &["b", "c", "e"], false),
    ];
    assert_eq!(ids(&diamond), ["b", "c", "d"]);
    // Nothing halted, nothing blocked.
    assert!(ids(&[node("a", &[], false), node("b", &["a"], false)]).is_empty());
    // A halted plan downstream of a halted one is blocked as well; the caller lets its own status win.
    assert_eq!(ids(&[node("a", &[], true), node("b", &["a"], true)]), ["b"]);
}

#[test]
fn a_cycle_fails_closed() {
    // a <-> b is a cycle, c is downstream of it, d is not.
    let nodes = [
        node("a", &["b"], false),
        node("b", &["a"], false),
        node("c", &["b"], false),
        node("d", &[], false),
    ];
    assert_eq!(ids(&nodes), ["a", "b", "c"]);
    // A self-loop is a cycle, and a halted plan on one is the blocker, not blocked.
    assert_eq!(ids(&[node("a", &["a"], false)]), ["a"]);
    assert!(ids(&[node("a", &["a"], true)]).is_empty());
}

#[test]
fn a_dependency_outside_the_phase_is_dropped() {
    // `x` is not a node: the caller filters other phases' plans out, and what is left unresolved is ignored.
    assert!(ids(&[node("a", &["x"], false)]).is_empty());
}
