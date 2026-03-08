use specforge_graph::Graph;
use std::fmt::Write;

pub fn emit_dot(graph: &Graph) -> String {
    let mut out = String::new();
    writeln!(out, "digraph specforge {{").unwrap();
    writeln!(out, "  rankdir=LR;").unwrap();
    writeln!(out, "  node [shape=box];").unwrap();

    for node in graph.nodes() {
        let label = match &node.title {
            Some(title) => format!("{}\\n{}", node.id.raw, title),
            None => node.id.raw.clone(),
        };
        writeln!(out, "  \"{}\" [label=\"{}\"];", node.id.raw, label).unwrap();
    }

    let mut edges: Vec<_> = graph.edges().to_vec();
    edges.sort_by(|a, b| (&a.source, &a.target, &a.label).cmp(&(&b.source, &b.target, &b.label)));

    for edge in &edges {
        writeln!(
            out,
            "  \"{}\" -> \"{}\" [label=\"{}\"];",
            edge.source, edge.target, edge.label
        )
        .unwrap();
    }

    writeln!(out, "}}").unwrap();
    out
}
