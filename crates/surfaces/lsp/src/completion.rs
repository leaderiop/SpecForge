//! What completes at a cursor: the items of a [`CompletionSite`], read from
//! the project view's registries and graph (ADR 0023). Where the cursor is
//! is the document module's to say.

use specforge_common::{Sym, structural};
use specforge_graph::Reading;
use specforge_ops::authoring::edit::{Skeleton, Skeletons, SlotOf};
use specforge_ops::authoring::format::Place;
use specforge_ops::read::infer::{KindGuide, guide};
use specforge_ops::read::navigate::{EntityQuery, MatchScope, find_entities};
use specforge_ops::view::ProjectView;
use specforge_protocol_types::FieldDescriptor;
use specforge_registry::{FieldRegistry, FieldType};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, InsertReplaceEdit,
    InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind, TextEdit,
};

use crate::document::{CompletionSite, Cursor};

/// The completion items at `cursor`. Each carries an edit over the word
/// under the cursor: an insert-and-replace edit when the client
/// supports one (`insert_replace`), else a plain edit over the word's start
/// to the cursor; its `filterText` is its label. A keyword's and a field's snippet is the skeleton the typed create
/// gives it in the file at `place` (ADR 0191).
pub fn items(
    cursor: &Cursor<'_>,
    insert_replace: bool,
    view: &ProjectView,
    place: Place<'_>,
) -> Vec<CompletionItem> {
    let registries = view.registries();
    let edit = cursor.word_edit();
    let holder = holder(cursor, view, place);
    let mut items = match cursor.completion() {
        CompletionSite::Keywords { prefix } => keywords(prefix, view, &Skeletons::at(view, place)),
        CompletionSite::Fields { kind, prefix } => {
            let skeletons = Skeletons::at(view, place);
            let written = cursor.written_fields();
            let mut fields = registries.fields().fields_for_kind(kind);
            fields.sort_by(|a, b| a.declared().name.cmp(&b.declared().name));
            fields
                .into_iter()
                .filter(|field| starts_with(&field.declared().name, prefix))
                .filter(|field| !written.contains(&field.declared().name.as_str()))
                .map(|field| {
                    let name = &field.declared().name;
                    let insert_text = skeletons.member(kind, name).map(|s| snippet(&s));
                    CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::FIELD),
                        detail: field.declared().description.clone(),
                        insert_text_format: insert_text.as_ref().map(|_| InsertTextFormat::SNIPPET),
                        // The skeleton is relative to its line: the client adds the line's own lead.
                        insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
                        insert_text,
                        ..Default::default()
                    }
                })
                .collect()
        }
        CompletionSite::VerifyKind { kind, prefix } => registries
            .kinds()
            .get(kind)
            .filter(|entry| entry.supports_verify)
            .map(|entry| {
                entry
                    .allowed_verify_kinds
                    .iter()
                    .filter(|verify| starts_with(verify, prefix))
                    .map(|verify| CompletionItem {
                        label: verify.clone(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        detail: Some(format!("verify kind of {kind}")),
                        ..Default::default()
                    })
                    .collect()
            })
            .unwrap_or_default(),
        CompletionSite::Members { kind, path, prefix } => {
            let written = cursor.written_fields();
            registries
                .fields()
                .get_path(kind, &path)
                .map(|(declared, _)| {
                    declared
                        .members
                        .iter()
                        .filter(|member| starts_with(&member.name, prefix))
                        .filter(|member| !written.contains(&member.name.as_str()))
                        .map(|member| CompletionItem {
                            label: member.name.clone(),
                            kind: Some(CompletionItemKind::FIELD),
                            detail: member.description.clone(),
                            ..Default::default()
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        CompletionSite::ListItem {
            kind,
            field,
            prefix,
            written,
        } => {
            let entry = registries.fields().get_path(kind, &field);
            match registries.field_forms().reading_at(kind, &field) {
                // A reference list: the ids of its target kind, or of any kind of its union.
                Reading::Declared(form) if form.field_type() == FieldType::ReferenceList => {
                    let kinds = entry.map(|(d, _)| target_kinds(d)).unwrap_or_default();
                    entity_ids(view, prefix, &kinds, holder)
                }
                // A string list's items are strings; the only entities it
                // names are the refs a scheme ref ID names.
                Reading::Declared(form) if form.field_type() == FieldType::StringList => {
                    entity_ids(view, prefix, &[structural::REF], holder)
                }
                // An enum list's words it does not hold yet.
                Reading::Declared(form) if form.field_type() == FieldType::EnumList => form
                    .enum_values()
                    .iter()
                    .filter(|word| starts_with(word, prefix) && !written.contains(&word.as_str()))
                    .map(|word| CompletionItem {
                        label: word.clone(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        ..Default::default()
                    })
                    .collect(),
                Reading::Declared(_) => Vec::new(),
                // An undeclared field, or a member of an open-fields kind
                // (whose lists name types): every id.
                Reading::Member | Reading::Undeclared if !field.contains('.') => {
                    entity_ids(view, prefix, &[], holder)
                }
                Reading::Member | Reading::Undeclared => Vec::new(),
            }
        }
        CompletionSite::Value {
            kind,
            field,
            prefix,
        } => {
            // A scope field is read from the project, not from its holder (ADR 0284).
            let from = holder.filter(|_| {
                view.registries()
                    .graph_inputs()
                    .scopes
                    .get(kind)
                    .map(String::as_str)
                    != Some(field.as_str())
            });
            value(registries.fields(), kind, &field, prefix, view, from)
        }
        CompletionSite::Nothing => Vec::new(),
    };
    for item in &mut items {
        let new_text = item
            .insert_text
            .clone()
            .unwrap_or_else(|| item.label.clone());
        item.text_edit = Some(if insert_replace {
            CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
                new_text,
                insert: edit.insert,
                replace: edit.replace,
            })
        } else {
            CompletionTextEdit::Edit(TextEdit {
                range: edit.insert,
                new_text,
            })
        });
        item.filter_text = Some(item.label.clone());
    }
    items
}

/// The path of the entity block around the cursor: the one of its kind and name whose block holds the cursor's line.
fn holder(cursor: &Cursor<'_>, view: &ProjectView, place: Place<'_>) -> Option<Sym> {
    let at = cursor.entity()?;
    let name = at.id?;
    let line = cursor.position().line as usize + 1;
    let Place::File(file) = place else {
        return None;
    };
    let nodes = view.graph().nodes();
    let same: Vec<_> = nodes
        .iter()
        .filter(|node| {
            node.kind.raw == at.kind
                && node.name() == name
                && file.ends_with(node.source_span.file.as_str())
        })
        .collect();
    match same.as_slice() {
        [one] => Some(one.id.raw),
        _ => same
            .iter()
            .find(|n| n.source_span.start_line <= line && line <= n.source_span.end_line)
            .map(|n| n.id.raw),
    }
}

/// Whether `label` starts with `prefix`, ignoring case.
fn starts_with(label: &str, prefix: &str) -> bool {
    label.to_lowercase().starts_with(&prefix.to_lowercase())
}

/// The top level's keywords: `use` and every registered kind, each kind
/// scaffolding its required fields and documented from its inference guide.
/// `define` is a reserved word whose blocks register nothing (W143, ADR
/// 0005): never suggested.
fn keywords(prefix: &str, view: &ProjectView, skeletons: &Skeletons) -> Vec<CompletionItem> {
    let kinds = view.registries().kinds();
    let guide = guide(view);
    let mut keywords: Vec<String> = kinds.keywords().cloned().collect();
    keywords.push(structural::USE.into());
    keywords.push(structural::DEFAULTS.into());
    keywords.sort();
    keywords.dedup();
    keywords
        .into_iter()
        .filter(|keyword| starts_with(keyword, prefix))
        .map(|keyword| {
            let (detail, snippet) = match kinds.get(&keyword) {
                Some(entry) => (
                    Some(entry.source_extension.clone()),
                    skeletons.block(&keyword).map(|s| snippet(&s)),
                ),
                None => (None, None),
            };
            CompletionItem {
                kind: Some(CompletionItemKind::KEYWORD),
                detail,
                documentation: guide.kind(&keyword).and_then(kind_documentation),
                insert_text_format: snippet.as_ref().map(|_| InsertTextFormat::SNIPPET),
                insert_text: snippet,
                label: keyword,
                ..Default::default()
            }
        })
        .collect()
}

/// A kind's markdown documentation: its description, then its inference
/// guide under "**Inferring it from code**"; none when it has neither.
fn kind_documentation(kind: &KindGuide) -> Option<Documentation> {
    let mut sections: Vec<String> = Vec::new();
    if let Some(description) = kind.description.filter(|d| !d.is_empty()) {
        sections.push(description.to_string());
    }
    if !kind.guide.is_empty() {
        sections.push(format!("**Inferring it from code**\n\n{}", kind.guide));
    }
    if sections.is_empty() {
        return None;
    }
    Some(Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: sections.join("\n\n"),
    }))
}

/// A field's single value, completed from its declared type: a reference's
/// entity IDs (of its target kind), an enum's declared values, `true` and
/// `false` for a boolean; entity IDs for a field the registry does not
/// type (it may hold a reference); nothing for a string, an integer, a
/// list or a block.
fn value(
    fields: &FieldRegistry,
    kind: &str,
    field: &str,
    prefix: &str,
    view: &ProjectView,
    from: Option<Sym>,
) -> Vec<CompletionItem> {
    let Some((declared, form)) = fields.get_path(kind, field) else {
        // A member path no field declares (an open block's entry) completes nothing.
        return if field.contains('.') {
            Vec::new()
        } else {
            entity_ids(view, prefix, &[], from)
        };
    };
    let constant = |label: &str, item_kind| CompletionItem {
        label: label.to_string(),
        kind: Some(item_kind),
        detail: declared.description.clone(),
        ..Default::default()
    };
    match form.field_type() {
        FieldType::Reference => entity_ids(view, prefix, &target_kinds(declared), from),
        FieldType::Enum => form
            .enum_values()
            .iter()
            .filter(|value| starts_with(value, prefix))
            .map(|value| constant(value, CompletionItemKind::ENUM_MEMBER))
            .collect(),
        FieldType::Bool => ["true", "false"]
            .into_iter()
            .filter(|value| starts_with(value, prefix))
            .map(|value| constant(value, CompletionItemKind::KEYWORD))
            .collect(),
        _ => Vec::new(),
    }
}

/// The kinds a reference field may name: its target kind, or each kind of its union; none when it names any.
fn target_kinds(declared: &FieldDescriptor) -> Vec<&str> {
    declared
        .target_kind
        .iter()
        .map(String::as_str)
        .chain(declared.target_kinds.iter().map(String::as_str))
        .collect()
}

/// The entity IDs matching `prefix`, of `kinds` when any are given, ranked
/// as completion, workspace symbols and MCP search rank them.
/// `from` is the block the id is written in (none: the project): the shortest spelling that resolves from it is what
/// completion inserts (ADR 0284).
fn entity_ids(
    view: &ProjectView,
    prefix: &str,
    kinds: &[&str],
    from: Option<Sym>,
) -> Vec<CompletionItem> {
    let query = EntityQuery {
        kinds,
        ..EntityQuery::new(prefix, MatchScope::Names)
    };
    find_entities(view.graph(), &query)
        .into_iter()
        .enumerate()
        .map(|(rank, found)| {
            let node = found.node;
            let kind = node.kind.raw.as_str();
            let detail = node
                .title
                .as_ref()
                .map(|t| format!("{kind} — {t}"))
                .unwrap_or_else(|| kind.to_string());
            let name = node.name();
            let path = node.id.raw;
            // An entity of a scope shows its path and inserts the shortest spelling from the cursor's block.
            let (detail, insert_text) = if path.as_str() == name {
                (detail, None)
            } else {
                let spelled = view.graph().scopes().spell(from, path, &|_| true);
                (format!("{path} · {detail}"), Some(spelled))
            };
            CompletionItem {
                label: name.to_string(),
                insert_text,
                kind: Some(CompletionItemKind::REFERENCE),
                detail: Some(detail),
                // C4-06: preserve the server's ranking in the editor.
                sort_text: Some(format!("{rank:04}")),
                ..Default::default()
            }
        })
        .collect()
}

/// `skeleton` as an LSP snippet: each slot a tabstop numbered in text order (`End` is `$0`), its placeholder text,
/// when it has one, the tabstop's default; `$` and `\` escaped in the text, `}` too in a default.
fn snippet(skeleton: &Skeleton) -> String {
    let escaped = |text: &str, default: bool| {
        let mut out = String::new();
        for c in text.chars() {
            if matches!(c, '$' | '\\') || (default && c == '}') {
                out.push('\\');
            }
            out.push(c);
        }
        out
    };
    let (mut out, mut from, mut n) = (String::new(), 0, 0);
    for slot in &skeleton.slots {
        out.push_str(&escaped(&skeleton.text[from..slot.at.start], false));
        let placeholder = &skeleton.text[slot.at.clone()];
        if slot.of == SlotOf::End {
            out.push_str("$0");
        } else {
            n += 1;
            if placeholder.is_empty() {
                out.push_str(&format!("${n}"));
            } else {
                out.push_str(&format!("${{{n}:{}}}", escaped(placeholder, true)));
            }
        }
        from = slot.at.end;
    }
    out.push_str(&escaped(&skeleton.text[from..], false));
    out
}
