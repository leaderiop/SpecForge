use specforge_graph::Graph;
use specforge_registry::{FieldRegistry, KindRegistry};
use std::collections::BTreeMap;

/// Returns markdown-formatted hover content for an entity.
///
/// Shows:
/// - Entity kind, ID, and title
/// - Extension source (from KindRegistry, if available)
/// - **References** (outgoing edges): grouped by field label, listing target IDs
/// - **Referenced by** (incoming edges): grouped by "source_kind via label", listing source IDs
/// - **Fields** (from FieldRegistry, if available): field names with types
pub fn hover_info(graph: &Graph, entity_id: &str) -> Option<String> {
    hover_info_with_registries(graph, entity_id, None, None)
}

/// Hover with optional registry metadata.
pub fn hover_info_with_registries(
    graph: &Graph,
    entity_id: &str,
    kind_registry: Option<&KindRegistry>,
    field_registry: Option<&FieldRegistry>,
) -> Option<String> {
    let node = graph.node(entity_id)?;

    let title = node
        .title
        .as_deref()
        .map(|t| format!(" — {t}"))
        .unwrap_or_default();

    let mut parts = vec![format!("**{}** `{}`{}", node.kind.raw, node.id.raw, title)];

    // Extension source from KindRegistry
    if let Some(kind_reg) = kind_registry
        && let Some(entry) = kind_reg.get(node.kind.raw.as_str()) {
            parts.push(format!("*from {}*", entry.source_extension));
    }

    // Outgoing edges: what this entity references, grouped by label
    let outgoing = graph.edges_from(entity_id);
    if !outgoing.is_empty() {
        let mut by_label: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in &outgoing {
            by_label.entry(edge.label.as_str()).or_default().push(edge.target.as_str());
        }
        let mut section = String::from("\n**References:**");
        for (label, targets) in &by_label {
            let ids: Vec<&str> = targets.to_vec();
            section.push_str(&format!("\n- {}: {}", label, ids.join(", ")));
        }
        parts.push(section);
    }

    // Incoming edges: what references this entity, grouped by source kind + label
    let incoming = graph.edges_to(entity_id);
    if !incoming.is_empty() {
        // Group by (source_kind, label) for clear display
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
        let mut section = String::from("\n**Referenced by:**");
        for ((kind, label), sources) in &by_kind_label {
            let ids: Vec<&str> = sources.to_vec();
            section.push_str(&format!("\n- {} ({}): {}", kind, label, ids.join(", ")));
        }
        parts.push(section);
    }

    // Fields from FieldRegistry
    if let Some(field_reg) = field_registry {
        let fields = field_reg.fields_for_kind(node.kind.raw.as_str());
        if !fields.is_empty() {
            let mut sorted: Vec<_> = fields.iter().map(|f| (&f.field_name, &f.field_type)).collect();
            sorted.sort_by_key(|(name, _)| *name);
            let mut section = String::from("\n**Fields:**");
            for (name, ftype) in &sorted {
                section.push_str(&format!("\n- `{}`: {}", name, format_field_type(ftype)));
            }
            parts.push(section);
        }
    }

    Some(parts.join("\n"))
}

/// Returns markdown-formatted hover content for a field name within an entity block.
pub fn hover_field_info(
    field_name: &str,
    entity_kind: &str,
    field_registry: &FieldRegistry,
) -> Option<String> {
    let entry = field_registry.get(entity_kind, field_name)?;

    let type_str = format_field_type(&entry.field_type);
    let mut parts = vec![format!("**`{}`** : {}", field_name, type_str)];

    if let Some(ref target) = entry.target_kind {
        parts.push(format!("→ **{}**", target));
    }

    if let Some(ref edge) = entry.edge {
        parts.push(format!("Edge: `{}`", edge));
    }

    if entry.required {
        parts.push("*required*".to_string());
    }

    parts.push(format!("*from {}*", entry.source_extension));

    Some(parts.join("  \n"))
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
