//! W159: a value written in the other form than its field's declared type
//! gives it (ADR 0147); a value a defaults block writes once, at the default, naming
//! `defaults <kind>` (ADR 0224).

use std::collections::HashSet;

use specforge_common::{Diagnostic, SourceSpan, codes};
use specforge_graph::FieldForm;
use specforge_parser::Written;
use specforge_protocol_types::FieldType;

use crate::entity::{EntityRecord, FieldRecord};
use crate::{FieldRegistry, KindRegistry};

/// W159 for every value written in the other form than its field's, one per
/// value, at the value; a field E061 refuses gets none (the error is the one
/// to fix first). A value a defaults block writes reaches each block that takes it at the default's own span
/// (ADR 0149 D9): one written value, one W159, naming `defaults <kind>` (ADR 0224).
pub(super) fn off_form(
    entities: &[EntityRecord],
    kinds: &KindRegistry,
    fields: &FieldRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut defaults_reported: HashSet<&SourceSpan> = HashSet::new();
    for record in entities {
        if !kinds.contains(&record.kind) {
            continue;
        }
        for field in &record.fields {
            let Some(declared) = fields.get(&record.kind, &field.key) else {
                continue;
            };
            if field.rewrites.is_empty() || declared.form().admit(field.shape, &field.text).is_err()
            {
                continue;
            }
            for rewrite in &field.rewrites {
                // A member E061 refuses gets none, as a field does.
                if rewrite.field != field.key
                    && refused_member(record, field, &rewrite.field, fields)
                {
                    continue;
                }
                let holder = if inherited(&record.span, &rewrite.span) {
                    if !defaults_reported.insert(&rewrite.span) {
                        continue;
                    }
                    format!("defaults {}", record.kind)
                } else {
                    format!("{} '{}'", record.kind, record.id)
                };
                let one_text = rewrite.listed && rewrite.text.contains('\n');
                let (what, fix) = match (rewrite.listed, rewrite.to) {
                    (true, _) if one_text => (
                        "a list written as one text",
                        "write each line as its own item",
                    ),
                    (true, _) => ("a list", "write it in brackets"),
                    (_, Written::Bare) => ("a word", "write it bare"),
                    (_, Written::Literal) => (literal_what(declared.form()), "write it bare"),
                    _ => ("text", "quote it"),
                };
                let is_member = rewrite.field != field.key;
                let item = match (is_member, !rewrite.listed && field.shape.is_list()) {
                    (true, _) => "member",
                    (false, true) => "an item of field",
                    (false, false) => "field",
                };
                let written_as = rewrite.field.rsplit('.').next().unwrap_or(&rewrite.field);
                let mut diagnostic = Diagnostic::new(
                    codes::W159,
                    format!(
                        "{item} '{}' of {holder} is {what}: {fix}, as {written_as} {}",
                        rewrite.field, rewrite.value
                    ),
                )
                .with_span(rewrite.span.clone());
                diagnostic.suggestion = Some(
                    if one_text {
                        "split the text into one item per step; format cannot tell where an item ends"
                    } else {
                        "run specforge format (or format the document) to write every value in its field's form"
                    }
                    .to_string(),
                );
                diagnostics.push(diagnostic);
            }
        }
    }
    diagnostics
}

/// Whether the member `path` (written) of `field` is one E061 refuses.
fn refused_member(
    record: &EntityRecord,
    field: &FieldRecord,
    path: &str,
    fields: &FieldRegistry,
) -> bool {
    let Some(member) = field.members.iter().find(|m| m.key == path) else {
        return false;
    };
    fields
        .get_path(&record.kind, &member.declared)
        .is_some_and(|(_, form)| form.admit(member.shape, &member.text).is_err())
}

/// Whether `value` lies outside `block`: an inherited entry keeps its default's spans (ADR 0149 D9).
fn inherited(block: &SourceSpan, value: &SourceSpan) -> bool {
    value.file != block.file
        || (value.start_line, value.start_col) < (block.start_line, block.start_col)
        || (value.end_line, value.end_col) > (block.end_line, block.end_col)
}

/// What a field written bare as a literal holds, for W159's wording.
fn literal_what(form: &FieldForm) -> &'static str {
    match form.field_type() {
        FieldType::Bool => "true or false",
        FieldType::Date => "a date",
        _ => "a number",
    }
}
