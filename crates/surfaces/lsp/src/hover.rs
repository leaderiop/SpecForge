//! The hover's markdown: the diagnostics under the cursor, an entity's
//! facts (the inspect read view, `specforge_ops::read::inspect`) and a field's
//! help. Rendering only: the facts come from the read view, so the hover
//! and MCP `specforge.inspect` cannot disagree (ADR 0015, "Inspect").

use specforge_common::{Codebook, Diagnostic, SourceSpan};
use specforge_ops::OpError;
use specforge_ops::read::coverage::STATUS;
use specforge_ops::read::inspect::{EntityCoverage, EntityFacts, obligations_table, quote};
use specforge_parser::{FieldValue, ListItem, TypeMember, Written};
use specforge_project::coverage::Status;
use specforge_registry::FieldRegistry;
use std::collections::BTreeMap;
use tower_lsp::lsp_types::{Position, Range};

/// The rule between the sections of a hover.
pub const SECTION: &str = "\n\n---\n\n";

/// The published diagnostics whose range (`range_of` their span; one without a range is under no
/// cursor) holds `position`, in published order.
pub fn diagnostics_at(
    published: &[Diagnostic],
    range_of: impl Fn(&SourceSpan) -> Option<Range>,
    position: Position,
) -> Vec<&Diagnostic> {
    let at = (position.line, position.character);
    published
        .iter()
        .filter(|diag| {
            diag.span.as_ref().and_then(&range_of).is_some_and(|range| {
                (range.start.line, range.start.character) <= at
                    && at <= (range.end.line, range.end.character)
            })
        })
        .collect()
}

/// Markdown for the diagnostics `shown` under the cursor
/// ([`diagnostics_at`]): each code with the code book's title, the message,
/// its explanation and its docs link (when it has one); a code the book
/// doesn't have shows its code, the extension that reported it and the
/// message. `None` when none is.
pub fn diagnostics(shown: &[&Diagnostic], codes: &Codebook) -> Option<String> {
    let sections: Vec<String> = shown
        .iter()
        .map(|diag| match codes.describes(&diag.code, diag.origin()) {
            Some(entry) => {
                let docs = entry
                    .docs
                    .map(|href| format!("\n\n[Documentation]({href})"))
                    .unwrap_or_default();
                format!(
                    "**{}** · {}\n\n{}\n\n{}{docs}",
                    entry.code, entry.title, diag.message, entry.explanation
                )
            }
            // An extension's own uncatalogued code, or one it may not use
            // (W150): no code book entry describes it.
            None => match diag.origin() {
                Some(extension) => format!(
                    "**{}** · reported by '{extension}'\n\n{}",
                    diag.code, diag.message
                ),
                None => format!("**{}**\n\n{}", diag.code, diag.message),
            },
        })
        .collect();
    (!sections.is_empty()).then(|| sections.join(SECTION))
}

/// Markdown for an entity's facts:
/// - its kind, ID and title; its kind's description, declaring extension
///   and badges (`testable` is the standing inspect reports); the
///   statement its extension declares headline, quoted whole;
/// - **Coverage**: its coverage, or why it does not count, or that the
///   recorded report cannot be read. While the project rebuilds it is the
///   last complete compile's. None for an entity of a
///   kind that is not testable and that declares no obligations;
/// - **Refers to**: its references, grouped by field;
/// - **Referenced by**: the references to it, grouped by the referencing
///   kind and field;
/// - **Fields**: its field values but the headline (`written` reads the word a span holds: an integer read from
///   another word shows it beside its value);
/// - **Methods**: its methods' signatures, one per line (ADR 0222);
/// - **Obligations**: its `verify` statements, the review document's table (ADR 0273);
/// - **Diagnostics**: the diagnostics about it that are not among those
///   `shown` for the cursor, each code linked to its docs page.
pub fn entity(
    facts: &EntityFacts,
    coverage: &Result<EntityCoverage, OpError>,
    shown: &[&Diagnostic],
    codes: &Codebook,
    written: &dyn Fn(&SourceSpan) -> Option<String>,
) -> String {
    let node = facts.node;
    let title = node
        .title
        .as_deref()
        .map(|t| format!(" — {t}"))
        .unwrap_or_default();

    // Section 1: Header + description + extension badges. The header
    // carries no editor markup: the icon a kind declares (a SymbolKind name)
    // reaches editors through document and workspace symbols, and a client
    // adds its own icon (ADR 0015, I4).
    let mut header_section = format!("**{}** `{}`{}", node.kind.raw, node.id.raw, title);
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
    if let Some(headline) = &facts.headline {
        header_section.push_str(&format!("\n\n{}", quote(&headline.text)));
    }

    let mut sections: Vec<String> = vec![header_section];

    // Section 2: its coverage
    sections.push(format!("**Coverage** · {}", coverage_line(facts, coverage)));

    // Section 3: its references (Refers to), by field
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

    // Section 4: the references to it (Referenced by), by kind and field
    let incoming = &facts.references.incoming;
    if !incoming.is_empty() {
        let mut by_kind_field: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
        for reference in incoming {
            let kind = reference.peer_kind.as_str();
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

    // Section 5: Fields, but the headline, which the summary quotes
    let fields: Vec<String> = node
        .fields
        .entries()
        .iter()
        .filter(|entry| facts.headline.as_ref().is_none_or(|h| h.field != entry.key))
        .map(|entry| {
            let key = entry.key.as_str();
            let annotations = facts.members.annotations.get(key).into_iter().flatten();
            let noted: String = annotations.map(|a| format!(" {}", a.spelled())).collect();
            let value = format_field_value(&entry.value);
            // An integer read from another word (`2-1`, `007`) shows the word beside its value (ADR 0283).
            let value = match (&entry.value, entry.value_span.as_ref().and_then(written)) {
                (FieldValue::Integer(n), Some(word)) if word != n.to_string() => {
                    format!("{value} (written {word})")
                }
                _ => value,
            };
            format!("- `{key}` = {value}{noted}")
        })
        .collect();
    if !fields.is_empty() {
        sections.push(format!("**Fields**\n{}", fields.join("\n")));
    }

    // Section 5a: its methods, one signature per line
    if !facts.members.methods.is_empty() {
        let lines: Vec<String> = facts
            .members
            .methods
            .iter()
            .map(|m| format!("- `{}`", m.signature()))
            .collect();
        sections.push(format!("**Methods**\n{}", lines.join("\n")));
    }

    // Section 5b: its obligations, apart from its fields
    sections.extend(
        obligations_table(facts.obligations).map(|rows| format!("**Obligations**\n\n{rows}")),
    );

    // Section 6: the diagnostics about it the cursor's do not already show
    let listed: Vec<String> = facts
        .diagnostics
        .iter()
        .filter(|diagnostic| !shown.contains(diagnostic))
        .map(|diagnostic| {
            let docs = codes
                .describes(&diagnostic.code, diagnostic.origin())
                .and_then(|entry| entry.docs);
            let code = match docs {
                Some(href) => format!("[**{}**]({href})", diagnostic.code),
                None => format!("**{}**", diagnostic.code),
            };
            format!("- {code} {}", diagnostic.message)
        })
        .collect();
    if !listed.is_empty() {
        sections.push(format!(
            "**Diagnostics** *({})*\n{}",
            listed.len(),
            listed.join("\n")
        ));
    }

    sections.join(SECTION)
}

/// The Coverage line after `**Coverage** · `: the status `specforge.inspect`
/// reports (ADR 0216), why an entity that does not count does not, then its
/// proof when it counts or declares obligations; the report's error, for
/// every entity, when the recorded report cannot be read.
fn coverage_line(facts: &EntityFacts, coverage: &Result<EntityCoverage, OpError>) -> String {
    let coverage = match coverage {
        Ok(coverage) => coverage,
        Err(error) => {
            return format!(
                "the recorded test report cannot be read ({}): {error}",
                error.code
            );
        }
    };
    let mut line = format!("`{}`", STATUS.name_of(coverage.status));
    match coverage.status {
        Status::Exempt if facts.standing.obligated() => {
            line.push_str(" · it owes none (a union or an exempting field)")
        }
        Status::Exempt => line.push_str(" · its kind need not declare obligations"),
        Status::NotTestable => line.push_str(" · its kind is not testable, so it does not count"),
        Status::Covered | Status::Partial | Status::Uncovered => {}
    }
    let verdict = &coverage.verdict;
    if coverage.status.counts() || verdict.obligations > 0 {
        line.push_str(&format!(
            " · {}/{} obligations proven",
            verdict.proven, verdict.obligations
        ));
        if coverage.recorded {
            let plural = if verdict.tests == 1 { "" } else { "s" };
            line.push_str(&format!(" · {} test{plural}", verdict.tests));
            if verdict.failing > 0 {
                line.push_str(&format!(" · {} failing", verdict.failing));
            }
        } else {
            line.push_str(" · no test report recorded");
        }
    }
    line
}

/// Returns markdown-formatted hover content for a field name within an entity block, or, as `requires.phase`, for a
/// member of a typed block (ADR 0285).
pub fn hover_field_info(
    field_name: &str,
    entity_kind: &str,
    field_registry: &FieldRegistry,
) -> Option<String> {
    let (declared, form) = field_registry.get_path(entity_kind, field_name)?;
    let entry = field_registry.get(entity_kind, field_name.split('.').next()?)?;

    let type_str = form.label();

    // First line: field name + type, with the target kind, or the kinds of its union, on same line
    let targets: Vec<&str> = declared
        .target_kind
        .iter()
        .chain(&declared.target_kinds)
        .map(String::as_str)
        .collect();
    let first_line = if targets.is_empty() {
        format!("**`{}`** : {}", field_name, type_str)
    } else {
        format!(
            "**`{}`** : {} → **{}**",
            field_name,
            type_str,
            targets.join(" | ")
        )
    };

    let mut parts = vec![first_line];

    if let Some(ref desc) = declared.description {
        parts.push(desc.clone());
    }
    if let Some(ref default) = declared.default_value {
        parts.push(format!("Absent, it counts as `{default}`."));
    }

    // Edge and required on same line
    match (&declared.edge, declared.required) {
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

    parts.push(format!("*{}*", entry.source_extension()));

    Some(parts.join("  \n"))
}

/// A union's members as written, joined with ` | `.
fn join_members(members: &[TypeMember]) -> String {
    members
        .iter()
        .map(|m| m.text.as_str())
        .collect::<Vec<_>>()
        .join(" | ")
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
        FieldValue::TypeUnion(types) => join_members(types),
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
        FieldValue::List(items) => {
            // Every item as written: a quoted one in quotes. A list of only
            // quoted strings shows its first five.
            let shown = |item: &ListItem| match item.written {
                Written::Quoted => format!("\"{}\"", item.text),
                _ => item.text.clone(),
            };
            let all_quoted = items.iter().all(|item| item.written == Written::Quoted);
            if all_quoted && items.len() > 5 {
                let first: Vec<_> = items[..5].iter().map(shown).collect();
                format!("[{}, … +{}]", first.join(", "), items.len() - 5)
            } else {
                format!(
                    "[{}]",
                    items.iter().map(shown).collect::<Vec<_>>().join(", ")
                )
            }
        }
        FieldValue::VariantList(variants) => format!("[{}]", join_members(variants)),
        FieldValue::Block(map) => {
            let count = map.entries().len();
            format!("{{…}} ({} fields)", count)
        }
        FieldValue::Blocks(items) => format!("[{{…}} × {}]", items.len()),
    }
}
