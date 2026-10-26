use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Degree, Direction, Edge, Graph, Node};

fn make_node(id: &str) -> Node {
    let span = SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 1,
        start_col: 1,
        end_line: 1,
        end_col: 1,
    };
    Node::new(id, "behavior", span)
}

fn add(graph: &mut Graph, source: &str, target: &str) {
    graph.add_edge(Edge {
        source: Sym::new(source),
        target: Sym::new(target),
        label: Sym::new("l"),
    });
}

#[test]
fn degree_counts_the_edges_to_and_from_other_entities() {
    let mut graph = Graph::new();
    for id in ["a", "b", "c", "loop"] {
        graph.add_node(make_node(id));
    }
    add(&mut graph, "a", "b");
    add(&mut graph, "a", "a");
    add(&mut graph, "c", "a");
    add(&mut graph, "a", "ghost");
    add(&mut graph, "loop", "loop");
    assert_eq!(
        graph.degree("a"),
        Degree {
            incoming: 1,
            outgoing: 2
        }
    );
    assert_eq!(
        graph.degree("ghost"),
        Degree {
            incoming: 1,
            outgoing: 0
        }
    );
    assert_eq!(graph.degree("nobody"), Degree::default());
    assert!(graph.degree("loop").is_unconnected());
}

#[test]
fn edges_with_others_leaves_out_an_edge_to_itself() {
    let mut graph = Graph::new();
    for id in ["a", "b", "c", "loop"] {
        graph.add_node(make_node(id));
    }
    add(&mut graph, "a", "b");
    add(&mut graph, "a", "a");
    add(&mut graph, "c", "a");
    add(&mut graph, "a", "ghost");
    add(&mut graph, "loop", "loop");
    let pairs = |id: &str, direction| {
        graph
            .edges_with_others(id, direction)
            .map(|e| (e.source.as_str().to_string(), e.target.as_str().to_string()))
            .collect::<Vec<_>>()
    };
    let pair = |s: &str, t: &str| (s.to_string(), t.to_string());
    assert_eq!(pairs("a", Direction::Incoming), [pair("c", "a")]);
    assert_eq!(
        pairs("a", Direction::Outgoing),
        [pair("a", "b"), pair("a", "ghost")]
    );
    assert!(pairs("loop", Direction::Incoming).is_empty());
    assert!(pairs("loop", Direction::Outgoing).is_empty());
    for id in ["a", "b", "c", "loop", "ghost", "nobody"] {
        let degree = graph.degree(id);
        assert_eq!(
            degree.incoming,
            pairs(id, Direction::Incoming).len(),
            "{id}"
        );
        assert_eq!(
            degree.outgoing,
            pairs(id, Direction::Outgoing).len(),
            "{id}"
        );
    }
}
