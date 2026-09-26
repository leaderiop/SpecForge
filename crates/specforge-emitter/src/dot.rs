use specforge_graph::Graph;
use std::fmt::Write;

/// Escape a string for safe inclusion inside a DOT quoted string.
/// Titles and descriptions are user-controlled: unescaped quotes
/// terminate the label early, backslashes form invalid escapes, and
/// raw newlines split the statement (C13-04).
fn escape_dot(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

pub fn emit_dot(graph: &Graph) -> String {
    let mut out = String::new();
    writeln!(out, "digraph specforge {{").unwrap();
    writeln!(out, "  rankdir=LR;").unwrap();
    writeln!(out, "  node [shape=box];").unwrap();

    for node in graph.nodes() {
        let label = match &node.title {
            Some(title) => format!(
                "{}\\n{}",
                escape_dot(node.id.raw.as_str()),
                escape_dot(title)
            ),
            None => escape_dot(node.id.raw.as_str()),
        };
        writeln!(
            out,
            "  \"{}\" [label=\"{}\"];",
            escape_dot(node.id.raw.as_str()),
            label
        )
        .unwrap();
    }

    let mut edges: Vec<_> = graph.edges().to_vec();
    edges.sort_by(|a, b| (&a.source, &a.target, &a.label).cmp(&(&b.source, &b.target, &b.label)));

    for edge in &edges {
        writeln!(
            out,
            "  \"{}\" -> \"{}\" [label=\"{}\"];",
            escape_dot(edge.source.as_str()),
            escape_dot(edge.target.as_str()),
            escape_dot(edge.label.as_str())
        )
        .unwrap();
    }

    writeln!(out, "}}").unwrap();
    out
}

#[cfg(test)]
mod dot_escape_tests {
    use super::*;
    use specforge_common::{SourceSpan, Sym};
    use specforge_graph::{EntityId, EntityKind, FieldMap, Node};

    #[test]
    fn escapes_quotes_backslashes_and_newlines() {
        assert_eq!(
            escape_dot("say \"hi\"\\done\nnext"),
            "say \\\"hi\\\"\\\\done\\nnext"
        );
    }

    #[test]
    fn hostile_title_cannot_break_the_dot_statement() {
        let mut graph = Graph::new();
        let mut fields = FieldMap::new();
        fields.push(
            Sym::new("description"),
            specforge_parser::FieldValue::String("}\"; evil | ".to_string()),
        );
        graph.add_node(Node {
            id: EntityId {
                raw: Sym::new("evil\"; hack"),
            },
            kind: EntityKind {
                raw: Sym::new("behavior"),
            },
            title: Some("}\"; hack graph".to_string()),
            fields,
            source_span: SourceSpan {
                file: Sym::new("t.spec"),
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            },
            methods: Vec::new(),
        });
        let dot = emit_dot(&graph);
        let node_stmts = dot
            .lines()
            .filter(|l| l.trim_start().starts_with('"'))
            .count();
        assert_eq!(
            node_stmts, 1,
            "hostile strings must not split statements: {dot}"
        );
        assert!(
            dot.contains("\\\";"),
            "embedded quote must be escaped: {dot}"
        );
        assert!(
            dot.contains("\\n}"),
            "embedded newline must be escaped: {dot}"
        );
    }
}
