//! E082: a computed field written in a spec file (ADR 0287).

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::FieldWriter;

use crate::FieldRegistry;
use crate::entity::{EntityRecord, Origin};

/// E082 for each field or member a compute pass gives that the entity also writes, at the written value.
/// `computed_by(kind, declared)` names the pass (and its extension) that computes the field, for the message. A field
/// a verdict writes is judged alike (ADR 0290): `verdict_of(kind, declared)` names the judged rule that binds it.
pub(super) fn hand_written(
    entities: &[EntityRecord],
    fields: &FieldRegistry,
    computed_by: impl Fn(&str, &str) -> Option<String>,
    verdict_of: impl Fn(&str, &str) -> Option<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for record in entities {
        for field in &record.fields {
            for field in std::iter::once(field).chain(&field.members) {
                if matches!(field.origin, Origin::Computed | Origin::Verdict) {
                    continue;
                }
                let Some((_, form)) = fields.get_path(&record.kind, &field.declared) else {
                    continue;
                };
                let message = match form.writer() {
                    Some(FieldWriter::Computed) => {
                        let by = computed_by(&record.kind, &field.declared)
                            .map_or_else(String::new, |pass| format!(" by {pass}"));
                        format!(
                            "field '{}' of {} '{}' is computed{by}, and is not written in a spec file",
                            field.key, record.kind, record.id
                        )
                    }
                    Some(FieldWriter::Verdict) => {
                        let rule = verdict_of(&record.kind, &field.declared)
                            .map_or_else(String::new, |rule| format!(" of '{rule}'"));
                        format!(
                            "field '{}' of {} '{}' comes from the verdict{rule}, and is not written in a spec file",
                            field.key, record.kind, record.id
                        )
                    }
                    _ => continue,
                };
                diagnostics.push(
                    Diagnostic::new(codes::E082, message)
                    .with_span(
                        field
                            .value_span
                            .clone()
                            .unwrap_or_else(|| record.span.clone()),
                    )
                    .with_suggestion(
                        "delete the line: the pass or the verdict gives the value when you check",
                    ),
                );
            }
        }
    }
    diagnostics
}
