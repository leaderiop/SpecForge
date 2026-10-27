//! E082: a computed field written in a spec file (ADR 0287).

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::FieldWriter;

use crate::FieldRegistry;
use crate::entity::{EntityRecord, Origin};

/// E082 for each field or member a compute pass gives that the entity also writes, at the written value.
/// `computed_by(kind, declared)` names the pass (and its extension) that computes the field, for the message.
pub(super) fn hand_written(
    entities: &[EntityRecord],
    fields: &FieldRegistry,
    computed_by: impl Fn(&str, &str) -> Option<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for record in entities {
        for field in &record.fields {
            for field in std::iter::once(field).chain(&field.members) {
                if field.origin == Origin::Computed {
                    continue;
                }
                let Some((_, form)) = fields.get_path(&record.kind, &field.declared) else {
                    continue;
                };
                if form.writer() != Some(FieldWriter::Computed) {
                    continue;
                }
                let by = computed_by(&record.kind, &field.declared)
                    .map_or_else(String::new, |pass| format!(" by {pass}"));
                diagnostics.push(
                    Diagnostic::new(
                        codes::E082,
                        format!(
                            "field '{}' of {} '{}' is computed{by}, and is not written in a spec file",
                            field.key, record.kind, record.id
                        ),
                    )
                    .with_span(
                        field
                            .value_span
                            .clone()
                            .unwrap_or_else(|| record.span.clone()),
                    )
                    .with_suggestion("delete the line: the pass gives the value when you check"),
                );
            }
        }
    }
    diagnostics
}
