use std::fmt::Write;

use super::{GroupBy, ModelIntermediate, ModelOptions};

const COLOR_SOFTWARE: &str = "#4a90d9";
const COLOR_PRODUCT: &str = "#2ecc71";
const COLOR_GOVERNANCE: &str = "#e74c3c";
const COLOR_FORMAL: &str = "#9b59b6";
const COLOR_FALLBACK: &str = "#95a5a6";

pub fn render_dot(model: &ModelIntermediate, options: &ModelOptions) -> String {
    let mut out = String::new();

    writeln!(out, "digraph model {{").unwrap();
    writeln!(out, "  rankdir=LR;").unwrap();
    writeln!(out, "  node [shape=none, fontname=\"Helvetica\"];").unwrap();
    writeln!(out, "  edge [fontname=\"Helvetica\", fontsize=10];").unwrap();

    match options.group_by {
        GroupBy::Extension => render_grouped(model, &mut out),
        GroupBy::None => render_flat(model, &mut out),
    }

    // Edges
    for rel in &model.relationships {
        writeln!(
            out, "  {} -> {} [label=\"{}\\n[{}]\"];",
            rel.source, rel.target, rel.name, rel.cardinality
        ).unwrap();
    }

    writeln!(out, "}}").unwrap();

    out
}

fn render_grouped(model: &ModelIntermediate, out: &mut String) {
    for ext in &model.extensions {
        let entities: Vec<_> = model.entities.iter().filter(|e| e.extension == ext.name).collect();
        if entities.is_empty() {
            continue;
        }

        let color = extension_color(&ext.name);
        let cluster_id = ext.name.replace("@specforge/", "").replace('/', "_");

        writeln!(out).unwrap();
        writeln!(out, "  subgraph cluster_{} {{", cluster_id).unwrap();
        writeln!(out, "    label=\"{}\";", ext.name).unwrap();
        writeln!(out, "    style=dashed;").unwrap();
        writeln!(out, "    color=\"{}\";", color).unwrap();

        for entity in entities {
            render_entity(entity, color, out);
        }

        writeln!(out, "  }}").unwrap();
    }
}

fn render_flat(model: &ModelIntermediate, out: &mut String) {
    for entity in &model.entities {
        let color = extension_color(&entity.extension);
        render_entity(entity, color, out);
    }
}

fn render_entity(entity: &super::ModelEntity, color: &str, out: &mut String) {
    writeln!(out).unwrap();

    let has_contributions = entity.fields.iter().any(|f| f.contributed_by.is_some() || f.contribution.is_some());
    let colspan = if has_contributions { 5 } else { 3 };

    let header_label = if entity.enhanced_by.is_empty() {
        entity.name.clone()
    } else {
        format!("{} (+{})", entity.name, entity.enhanced_by.join(", +"))
    };

    if entity.fields.is_empty() {
        writeln!(out, "    {} [label=<", entity.name).unwrap();
        writeln!(out, "      <table border=\"1\" cellborder=\"0\" cellspacing=\"0\">").unwrap();
        writeln!(
            out, "        <tr><td bgcolor=\"{}\" colspan=\"{}\"><font color=\"white\"><b>{}</b></font></td></tr>",
            color, colspan, header_label
        ).unwrap();
        writeln!(out, "      </table>").unwrap();
        writeln!(out, "    >];").unwrap();
    } else {
        writeln!(out, "    {} [label=<", entity.name).unwrap();
        writeln!(out, "      <table border=\"1\" cellborder=\"0\" cellspacing=\"0\">").unwrap();
        writeln!(
            out, "        <tr><td bgcolor=\"{}\" colspan=\"{}\"><font color=\"white\"><b>{}</b></font></td></tr>",
            color, colspan, header_label
        ).unwrap();

        for field in &entity.fields {
            let name_str = if field.required || field.is_primary_key {
                format!("<b>{}</b>", field.name)
            } else {
                field.name.clone()
            };

            let marker = if field.is_primary_key {
                "PK".to_string()
            } else if let Some(ref target) = field.references {
                format!("-> {}", target)
            } else {
                String::new()
            };

            if has_contributions {
                let contribution = field.contribution.as_deref().unwrap_or("");
                let source = field.contributed_by.as_deref().unwrap_or("");
                writeln!(
                    out, "        <tr><td align=\"left\">{}</td><td>{}</td><td>{}</td><td><i>{}</i></td><td><font color=\"gray\">{}</font></td></tr>",
                    name_str, field.field_type, marker, contribution, source
                ).unwrap();
            } else {
                writeln!(
                    out, "        <tr><td align=\"left\">{}</td><td>{}</td><td>{}</td></tr>",
                    name_str, field.field_type, marker
                ).unwrap();
            }
        }

        writeln!(out, "      </table>").unwrap();
        writeln!(out, "    >];").unwrap();
    }
}

fn extension_color(name: &str) -> &str {
    match name {
        "@specforge/software" => COLOR_SOFTWARE,
        "@specforge/product" => COLOR_PRODUCT,
        "@specforge/governance" => COLOR_GOVERNANCE,
        "@specforge/formal" => COLOR_FORMAL,
        _ => COLOR_FALLBACK,
    }
}
