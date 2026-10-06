//! The hover's markdown: the diagnostics under the cursor, an entity's
//! facts (the inspect read view, `specforge_ops::inspect`) and a field's
//! help. Rendering only: the facts come from the read view, so the hover
//! and MCP `specforge.inspect` cannot disagree (ADR 0015, "Inspect").

use crate::document::LineIndex;
use specforge_common::Diagnostic;
use specforge_ops::inspect::EntityFacts;
use specforge_parser::FieldValue;
use specforge_registry::FieldRegistry;
use std::collections::BTreeMap;
use tower_lsp::lsp_types::Position;

/// The published diagnostics whose range (in the document `index`
/// indexes) holds `position`, in published order.
pub fn diagnostics_at<'d>(
    published: &'d [Diagnostic],
    index: &LineIndex,
    position: Position,
) -> Vec<&'d Diagnostic> {
    let at = (position.line, position.character);
    published
        .iter()
        .filter(|diag| {
            diag.span.as_ref().is_some_and(|span| {
                let range = index.range(span);
                (range.start.line, range.start.character) <= at
                    && at <= (range.end.line, range.end.character)
            })
        })
        .collect()
}

/// Markdown for the diagnostics `shown` under the cursor
/// ([`diagnostics_at`]): each code with the catalogue's title, the message,
/// the catalogue's explanation and the docs link; a code the catalogue
/// doesn't have shows its code and message only. `None` when none is.
pub fn diagnostics(shown: &[&Diagnostic]) -> Option<String> {
    let sections: Vec<String> = shown
        .iter()
        .map(|diag| match specforge_diagnostics::lookup(&diag.code) {
            Some(entry) => {
                let mut section = format!(
                    "**{}** · {}\n\n{}\n\n{}",
                    entry.code, entry.title, diag.message, entry.explanation
                );
                if let Some(href) = specforge_diagnostics::docs_href(entry.code) {
                    section.push_str(&format!("\n\n[Documentation]({href})"));
                }
                section
            }
            None => format!("**{}**\n\n{}", diag.code, diag.message),
        })
        .collect();
    (!sections.is_empty()).then(|| sections.join("\n\n---\n\n"))
}

/// Markdown for an entity's facts:
/// - its kind, ID and title; its kind's description, declaring extension
///   and badges (`testable` is the standing inspect reports);
/// - **Refers to**: its references, grouped by field;
/// - **Referenced by**: the references to it, grouped by the referencing
///   kind and field;
/// - **Fields**: its field values.
pub fn entity(facts: &EntityFacts) -> String {
    let node = facts.node;
    let title = node
        .title
        .as_deref()
        .map(|t| format!(" — {t}"))
        .unwrap_or_default();

    // Section 1: Header + description + extension badges
    let icon = facts
        .kind
        .and_then(|entry| entry.declared.lsp_icon.clone())
        .map(|i| format!("{i} "))
        .unwrap_or_default();
    let mut header_section = format!("{icon}**{}** `{}`{}", node.kind.raw, node.id.raw, title);
    if let Some(entry) = facts.kind {
        if let Some(ref desc) = entry.declared.description {
            header_section.push_str(&format!("\n\n{}", desc));
        }
        let mut ext_line = format!("*{}*", entry.source_extension);
        if facts.standing.testable {
            ext_line.push_str(" · `testable`");
        }
        if entry.supports_verify {
            ext_line.push_str(" · `verify`");
        }
        if entry.declared.singleton {
            ext_line.push_str(" · `singleton`");
        }
        header_section.push_str(&format!("\n{}", ext_line));
    }

    let mut sections: Vec<String> = vec![header_section];

    // Section 2: its references (Refers to), by field
    let outgoing = &facts.references.outgoing;
    if !outgoing.is_empty() {
        let mut by_field: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for reference in outgoing {
            by_field
                .entry(reference.field.as_str())
                .or_default()
                .push(reference.peer.as_str());
        }
        let mut section = format!("**Refers to** *({})*", outgoing.len());
        for (field, targets) in &by_field {
            section.push_str(&format!("\n- `{}` → {}", field, targets.join(", ")));
        }
        sections.push(section);
    }

    // Section 3: the references to it (Referenced by), by kind and field
    let incoming = &facts.references.incoming;
    if !incoming.is_empty() {
        let mut by_kind_field: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
        for reference in incoming {
            let kind = reference.peer_kind.map_or("unknown", |k| k.as_str());
            by_kind_field
                .entry((kind, reference.field.as_str()))
                .or_default()
                .push(reference.peer.as_str());
        }
        let mut section = format!("**Referenced by** *({})*", incoming.len());
        for ((kind, field), sources) in &by_kind_field {
            section.push_str(&format!(
                "\n- {} via `{}`: {}",
                kind,
                field,
                sources.join(", ")
            ));
        }
        sections.push(section);
    }

    // Section 4: Fields
    let fields: Vec<String> = node
        .fields
        .entries()
        .iter()
        .filter(|entry| entry.key.as_str() != "title")
        .map(|entry| format!("- `{}` = {}", entry.key, format_field_value(&entry.value)))
        .collect();
    if !fields.is_empty() {
        sections.push(format!("**Fields**\n{}", fields.join("\n")));
    }

    sections.join("\n\n---\n\n")
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
    let first_line = if let Some(ref target) = entry.declared.target_kind {
        format!("**`{}`** : {} → **{}**", field_name, type_str, target)
    } else {
        format!("**`{}`** : {}", field_name, type_str)
    };

    let mut parts = vec![first_line];

    if let Some(ref desc) = entry.declared.description {
        parts.push(desc.clone());
    }

    // Edge and required on same line
    match (&entry.declared.edge, entry.declared.required) {
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
            // At most 120 bytes, cut at the last character boundary at or
            // before byte 120: a cut inside a character would panic.
            let truncated = if s.len() > 120 {
                format!("{}…", &s[..s.floor_char_boundary(120)])
            } else {
                s.clone()
            };
            format!("\"{}\"", truncated)
        }
        FieldValue::Identifier(s) => format!("`{}`", s),
        FieldValue::TypeUnion(types) => types.join(" | "),
        FieldValue::Expression(exprs) => {
            if exprs.len() == 1 {
                format!("expr {{ {} }}", exprs[0])
            } else {
                format!("expr {{ {} bounds }}", exprs.len())
            }
        }
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        FieldValue::Date(d) => d.clone(),
        FieldValue::ReferenceList(refs) => {
            format!(
                "[{}]",
                refs.iter()
                    .map(|r| r.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        FieldValue::StringList(items) => {
            if items.len() <= 5 {
                format!(
                    "[{}]",
                    items
                        .iter()
                        .map(|s| format!("\"{}\"", s))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
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
            let items: Vec<_> = stmts
                .iter()
                .map(|v| format!("{}: {}", v.kind, v.description))
                .collect();
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
