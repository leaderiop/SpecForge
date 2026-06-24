use specforge_graph::Graph;
use specforge_parser::FieldValue;
use specforge_registry::{FieldRegistry, KindRegistry};
use std::collections::BTreeMap;

/// Returns markdown-formatted hover content for an entity.
///
/// Shows:
/// - Entity kind, ID, and title
/// - Extension source (from KindRegistry, if available)
/// - **References** (outgoing edges): grouped by field label, listing target IDs
/// - **Referenced by** (incoming edges): grouped by "source_kind via label", listing source IDs
/// - **Fields**: actual field values from the entity
pub fn hover_info(graph: &Graph, entity_id: &str) -> Option<String> {
    hover_info_with_registries(graph, entity_id, None, None)
}

/// Hover with optional registry metadata.
pub fn hover_info_with_registries(
    graph: &Graph,
    entity_id: &str,
    kind_registry: Option<&KindRegistry>,
    _field_registry: Option<&FieldRegistry>,
) -> Option<String> {
    let node = graph.node(entity_id)?;

    let title = node
        .title
        .as_deref()
        .map(|t| format!(" — {t}"))
        .unwrap_or_default();

    // Section 1: Header + description + extension badges
    let mut header_section = format!("**{}** `{}`{}", node.kind.raw, node.id.raw, title);

    if let Some(kind_reg) = kind_registry
        && let Some(entry) = kind_reg.get(node.kind.raw.as_str())
    {
        if let Some(ref desc) = entry.description {
            header_section.push_str(&format!("\n\n{}", desc));
        }

        let mut ext_line = format!("*{}*", entry.source_extension);
        if entry.testable {
            ext_line.push_str(" · `testable`");
        }
        if entry.supports_verify {
            ext_line.push_str(" · `verify`");
        }
        if entry.singleton {
            ext_line.push_str(" · `singleton`");
        }
        header_section.push_str(&format!("\n{}", ext_line));
    }

    let mut sections: Vec<String> = vec![header_section];

    // Section 2: Outgoing edges (References)
    let outgoing = graph.edges_from(entity_id);
    if !outgoing.is_empty() {
        let mut by_label: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in &outgoing {
            by_label
                .entry(edge.label.as_str())
                .or_default()
                .push(edge.target.as_str());
        }
        let total_count = outgoing.len();
        let mut section = format!("**References** *({})*", total_count);
        for (label, targets) in &by_label {
            let ids: Vec<&str> = targets.to_vec();
            section.push_str(&format!("\n- `{}` → {}", label, ids.join(", ")));
        }
        sections.push(section);
    }

    // Section 3: Incoming edges (Referenced by)
    let incoming = graph.edges_to(entity_id);
    if !incoming.is_empty() {
        let mut by_kind_label: BTreeMap<(String, &str), Vec<&str>> = BTreeMap::new();
        for edge in &incoming {
            let source_kind = graph
                .node(edge.source.as_str())
                .map(|n| n.kind.raw.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            by_kind_label
                .entry((source_kind, edge.label.as_str()))
                .or_default()
                .push(edge.source.as_str());
        }
        let total_count = incoming.len();
        let mut section = format!("**Referenced by** *({})*", total_count);
        for ((kind, label), sources) in &by_kind_label {
            let ids: Vec<&str> = sources.to_vec();
            section.push_str(&format!("\n- {} via `{}`: {}", kind, label, ids.join(", ")));
        }
        sections.push(section);
    }

    // Section 4: Fields
    let field_entries = node.fields.entries();
    if !field_entries.is_empty() {
        let mut has_fields = false;
        let mut section = String::from("**Fields**");
        for entry in field_entries {
            let key = entry.key.as_str();
            if key == "title" {
                continue;
            }
            has_fields = true;
            section.push_str(&format!("\n- `{}` = {}", key, format_field_value(&entry.value)));
        }
        if has_fields {
            sections.push(section);
        }
    }

    Some(sections.join("\n\n---\n\n"))
}

/// Returns markdown-formatted hover content for a field name within an entity block.
pub fn hover_field_info(
    field_name: &str,
    entity_kind: &str,
    field_registry: &FieldRegistry,
) -> Option<String> {
    let entry = field_registry.get(entity_kind, field_name)?;

    let type_str = format_field_type(&entry.field_type);

    // First line: field name + type, with optional target kind on same line
    let first_line = if let Some(ref target) = entry.target_kind {
        format!("**`{}`** : {} → **{}**", field_name, type_str, target)
    } else {
        format!("**`{}`** : {}", field_name, type_str)
    };

    let mut parts = vec![first_line];

    if let Some(ref desc) = entry.description {
        parts.push(desc.clone());
    }

    // Edge and required on same line
    match (&entry.edge, entry.required) {
        (Some(edge_name), true) => {
            parts.push(format!("Edge `{}` · *required*", edge_name));
        }
        (Some(edge_name), false) => {
            parts.push(format!("Edge `{}`", edge_name));
        }
        (None, true) => {
            parts.push("*required*".to_string());
        }
        (None, false) => {}
    }

    parts.push(format!("*{}*", entry.source_extension));

    Some(parts.join("  \n"))
}

fn format_field_value(fv: &FieldValue) -> String {
    match fv {
        FieldValue::String(s) => {
            let truncated = if s.len() > 120 {
                format!("{}…", &s[..120])
            } else {
                s.clone()
            };
            format!("\"{}\"", truncated)
        }
        FieldValue::Identifier(s) => format!("`{}`", s),
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        FieldValue::Date(d) => d.clone(),
        FieldValue::ReferenceList(refs) => format!("[{}]", refs.join(", ")),
        FieldValue::StringList(items) => {
            if items.len() <= 5 {
                format!("[{}]", items.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", "))
            } else {
                let shown: Vec<_> = items[..5].iter().map(|s| format!("\"{}\"", s)).collect();
                format!("[{}, … +{}]", shown.join(", "), items.len() - 5)
            }
        }
        FieldValue::VariantList(variants) => format!("[{}]", variants.join(" | ")),
        FieldValue::MixedList(items) => {
            let formatted: Vec<_> = items.iter().map(format_field_value).collect();
            format!("[{}]", formatted.join(", "))
        }
        FieldValue::Block(map) => {
            let count = map.entries().len();
            format!("{{…}} ({} fields)", count)
        }
        FieldValue::VerifyList(stmts) => {
            let items: Vec<_> = stmts.iter().map(|v| format!("{}: {}", v.kind, v.description)).collect();
            if items.len() <= 3 {
                items.join("; ")
            } else {
                format!("{}; … +{}", items[..3].join("; "), items.len() - 3)
            }
        }
    }
}

fn format_field_type(ft: &specforge_registry::ManifestFieldType) -> &'static str {
    match ft {
        specforge_registry::ManifestFieldType::String => "string",
        specforge_registry::ManifestFieldType::Integer => "integer",
        specforge_registry::ManifestFieldType::Bool => "bool",
        specforge_registry::ManifestFieldType::Enum(_) => "enum",
        specforge_registry::ManifestFieldType::StringList => "string_list",
        specforge_registry::ManifestFieldType::Reference => "reference",
        specforge_registry::ManifestFieldType::ReferenceList => "reference_list",
        specforge_registry::ManifestFieldType::Block => "block",
    }
}
