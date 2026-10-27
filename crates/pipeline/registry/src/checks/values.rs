//! E061: a field value that cannot be the type its extension declared.

use specforge_common::{Diagnostic, codes};

use specforge_graph::ValueShape;
use specforge_graph::field_form::spell;

use crate::entity::{EntityRecord, FieldRecord};
use crate::{FieldRegistry, KindRegistry};
use specforge_graph::FieldForm;

/// The type as a message names it: `integer`, `enum (a, b)`.
fn form_label(form: &FieldForm) -> String {
    let name = form.field_type().as_str();
    match form.enum_values() {
        [] => name.to_string(),
        values => format!("{name} ({})", values.join(", ")),
    }
}

/// E061 for every registered field whose (coerced) value its field form
/// refuses: a scalar that is not an integer, true/false or a declared enum
/// value; a list on a single-value field; a block or an
/// expression where the type never takes one (a reference or list field);
/// a scalar on a block field. Unregistered kinds and fields are left to
/// E024 / W020. Each is reported at the value's own
/// span, else the entity's.
pub(super) fn mistyped(
    entities: &[EntityRecord],
    kinds: &KindRegistry,
    fields: &FieldRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for record in entities {
        if !kinds.contains(&record.kind) {
            continue;
        }
        for field in &record.fields {
            // A field, then each typed member written inside it, by its written path (ADR 0285).
            for field in std::iter::once(field).chain(&field.members) {
                let Some((_, form)) = fields.get_path(&record.kind, &field.declared) else {
                    continue;
                };
                let Some(mismatch) = mismatch(form, field) else {
                    continue;
                };
                let label = form_label(form);
                let mut diagnostic = Diagnostic::new(
                    codes::E061,
                    format!(
                        "field '{}' of {} '{}' is declared {}, but was given {}",
                        field.key, record.kind, record.id, label, mismatch.given
                    ),
                )
                .with_span(
                    field
                        .value_span
                        .clone()
                        .unwrap_or_else(|| record.span.clone()),
                );
                diagnostic.suggestion = mismatch.suggestion;
                diagnostics.push(diagnostic);
            }
        }
    }
    diagnostics
}

struct Mismatch {
    given: String,
    suggestion: Option<String>,
}

fn mismatch(form: &FieldForm, field: &FieldRecord) -> Option<Mismatch> {
    if let Err(refusal) = form.admit(field.shape, &field.text) {
        return Some(Mismatch {
            given: describe(field),
            suggestion: refusal.suggestion,
        });
    }
    let (item, refusal) = form.refuse_item(field.items.as_deref()?)?;
    Some(Mismatch {
        given: item.to_string(),
        suggestion: refusal.suggestion,
    })
}

/// The value as the message quotes it: a string as a `.spec` string writes it
/// ([`spell::quoted`]).
fn describe(field: &FieldRecord) -> String {
    match field.shape {
        ValueShape::String => spell::quoted(&field.text),
        ValueShape::Identifier
        | ValueShape::Date
        | ValueShape::Integer
        | ValueShape::Boolean
        | ValueShape::TypeUnion => field.text.clone(),
        ValueShape::List | ValueShape::Variants => "a list".to_string(),
        ValueShape::Block => "a block".to_string(),
        ValueShape::Blocks => "a list of blocks".to_string(),
        ValueShape::Expression => "an expression".to_string(),
    }
}
