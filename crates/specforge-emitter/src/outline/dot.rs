use std::fmt::Write;

use super::{OutlineDetail, OutlineIntermediate, OutlineOptions};

const COLORS: &[(&str, &str)] = &[
    ("product", "#2ecc71"),
    ("software", "#4a90d9"),
    ("governance", "#e74c3c"),
    ("formal", "#9b59b6"),
];

pub fn render_dot(outline: &OutlineIntermediate, options: &OutlineOptions) -> String {
    let mut out = String::new();

    writeln!(out, "digraph extensions {{").unwrap();
    writeln!(out, "    rankdir=TB;").unwrap();
    writeln!(out, "    node [shape=record, style=filled, fontname=\"Helvetica\"];").unwrap();
    writeln!(out, "    edge [fontname=\"Helvetica\", fontsize=10];").unwrap();
    writeln!(out).unwrap();

    // Extension nodes
    for ext in &outline.extensions {
        let id = sanitize_id(&ext.name);
        let color = extension_color(&ext.name);
        let label = if options.detail == OutlineDetail::All {
            let kinds: Vec<&str> = ext.entity_kinds.iter().map(|k| k.keyword.as_str()).collect();
            format!(
                "{{ {} | {} | {} entities, {} edges | {} }}",
                ext.name,
                ext.version,
                ext.entity_kinds.len(),
                ext.edge_types.len(),
                kinds.join(", ")
            )
        } else {
            format!(
                "{{ {} | {} | {} entities, {} edges }}",
                ext.name,
                ext.version,
                ext.entity_kinds.len(),
                ext.edge_types.len()
            )
        };
        writeln!(
            out,
            "    {} [label=\"{}\", fillcolor=\"{}\", fontcolor=\"white\"];",
            id, label, color
        )
        .unwrap();
    }

    writeln!(out).unwrap();

    // Dependency edges (filtered by visibility mode)
    let visible_deps = super::filter_dependencies(&outline.dependencies, options.deps);
    for dep in &visible_deps {
        let from_id = sanitize_id(&dep.from);
        let to_id = sanitize_id(&dep.to);
        if dep.optional {
            writeln!(
                out,
                "    {} -> {} [label=\"optional {}\", style=dashed];",
                from_id, to_id, dep.version
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "    {} -> {} [label=\"depends {}\"];",
                from_id, to_id, dep.version
            )
            .unwrap();
        }
    }

    // Enhancement edges (dashed)
    for enh in &outline.enhancements {
        let from_id = sanitize_id(&enh.enhancer);
        let to_id = sanitize_id(&enh.owner);
        writeln!(
            out,
            "    {} -> {} [label=\"enhances {} (+{})\", style=dashed, color=\"#999999\"];",
            from_id, to_id, enh.target_kind, enh.field_count
        )
        .unwrap();
    }

    // Cross-extension edges (dotted red)
    for ce in &outline.cross_edges {
        let from_id = sanitize_id(&ce.owner_extension);
        let to_id = sanitize_id(&ce.target_extension);
        writeln!(
            out,
            "    {} -> {} [label=\"{}\", style=dotted, color=\"#e74c3c\"];",
            from_id, to_id, ce.edge_label
        )
        .unwrap();
    }

    writeln!(out, "}}").unwrap();
    out
}

fn sanitize_id(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '@')
        .map(|c| if c == '/' || c == '-' { '_' } else { c })
        .collect()
}

fn extension_color(name: &str) -> &'static str {
    for (key, color) in COLORS {
        if name.contains(key) {
            return color;
        }
    }
    "#95a5a6"
}
