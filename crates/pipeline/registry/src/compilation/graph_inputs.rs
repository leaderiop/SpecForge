//! What the graph build reads of the loaded declarations (ADR 0032,
//! amended): derived here, from the declarations and the field registry,
//! and nowhere else. The grammar parses values generically; the field
//! registry says what a field's value must be and which fields are edges.

use std::collections::{BTreeMap, BTreeSet};

use specforge_graph::{DerivedFrom, DerivedReference, FieldForms, GraphInputs};

use crate::{FieldRegistry, KindRegistry};

pub(super) fn derive(
    kinds: &KindRegistry,
    fields: &FieldRegistry,
    forms: &FieldForms,
    owners_of: impl Fn(&str) -> Vec<String>,
) -> GraphInputs {
    GraphInputs {
        target_owners: target_owners(forms, owners_of),
        field_forms: forms.clone(),
        derived_references: derived_references(fields),
        scopes: scopes(kinds),
        fact_schemes: fact_schemes(kinds),
    }
}

/// Each scheme a collector-written kind declares, and the kinds that declare it (ADR 0327 D6).
fn fact_schemes(kinds: &KindRegistry) -> BTreeMap<String, BTreeSet<String>> {
    let mut schemes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (kind, entry) in kinds.iter().filter(|(_, entry)| entry.is_collected()) {
        for scheme in entry.schemes() {
            schemes
                .entry(scheme.clone())
                .or_default()
                .insert(kind.clone());
        }
    }
    schemes
}

/// Each scoped kind's scope field (ADR 0284): the declared `scoped_by` the registry build kept.
fn scopes(kinds: &KindRegistry) -> BTreeMap<String, String> {
    kinds
        .iter()
        .filter_map(|(kind, entry)| Some((kind.clone(), entry.scoped_by.clone()?)))
        .collect()
}

/// For each target kind no loaded extension declares, every extension the candidate vocabulary
/// credits with it (`owners_of`, the lookup E024 reads), in its order, so I004 and E022 name each
/// extension E024 names (ADR 0117, ADR 0369). A kind nothing declares has no entry.
fn target_owners(
    forms: &FieldForms,
    owners_of: impl Fn(&str) -> Vec<String>,
) -> BTreeMap<String, Vec<String>> {
    forms
        .targets()
        .into_iter()
        .flat_map(|(_, target)| target.absent())
        .map(|kind| (kind.to_string(), owners_of(kind)))
        .filter(|(_, owners)| !owners.is_empty())
        .collect()
}

/// Every registered field's form, and the kinds that take any field
/// (`open_fields`): how the graph build stores a written value as the
/// field's declared type, and what each reader takes it to reference. The
/// values that still can't be the declared type are `RegistryBuild::check`'s
/// E061.
pub(super) fn field_forms(kinds: &KindRegistry, fields: &FieldRegistry) -> FieldForms {
    let forms = fields
        .iter()
        .map(|(kind, field, entry)| ((kind.to_string(), field.to_string()), entry.form().clone()))
        .collect();
    let member_kinds = kinds
        .iter()
        .filter(|(_, entry)| entry.declared.open_fields)
        .map(|(name, _)| name.clone())
        .collect();
    let declared = kinds.iter().map(|(name, _)| name.clone()).collect();
    FieldForms::new(forms, member_kinds, declared)
}

/// The registered reference fields whose edges the host derives
/// (`derived_from`). A field that names an unknown source or no target
/// kind derives nothing (the declaration check reports it as W021).
/// Sorted, so the build is deterministic.
fn derived_references(fields: &FieldRegistry) -> Vec<DerivedReference> {
    let mut derived: Vec<DerivedReference> = fields
        .iter()
        .filter(|(_, _, entry)| entry.field_type().is_reference())
        .filter_map(|(kind, field, entry)| {
            (!entry.form().target_kinds().is_empty()).then_some(())?;
            Some(DerivedReference {
                source_kind: kind.to_string(),
                field: field.to_string(),
                from: DerivedFrom::parse(entry.declared().derived_from.as_deref()?)?,
            })
        })
        .collect();
    derived.sort_by(|a, b| (&a.source_kind, &a.field).cmp(&(&b.source_kind, &b.field)));
    derived
}
