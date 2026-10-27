use super::build::{BuildPart, Reported};
use super::edge_types;
use crate::{
    EdgeRegistry, FieldProblem, FieldRegistry, FieldRegistryEntry, KindRegistry, KindRegistryEntry,
    UnknownFieldType,
};
use specforge_common::{Diagnostic, DiagnosticData, codes};
use specforge_protocol_types::{
    EdgeTypeDescriptor, EntityEnhancementDescriptor, EntityKindDescriptor, ExtensionDeclaration,
    FieldDescriptor, FieldType,
};
use std::collections::BTreeMap;

/// The keyword a kind's entities are written with: its declared keyword,
/// else its name.
pub(crate) fn keyword(kind: &EntityKindDescriptor) -> &str {
    kind.keyword.as_deref().unwrap_or(&kind.name)
}

/// Populate all three registries from the declarations, in load order
/// (ADR 0041), plus the diagnostics of doing so.
pub(crate) fn populate(
    declarations: &[ExtensionDeclaration],
) -> (KindRegistry, FieldRegistry, EdgeRegistry, Vec<Reported>) {
    let mut kind_reg = KindRegistry::new();
    let mut field_reg = FieldRegistry::new();
    let mut declared_edges = Vec::new();
    let mut diagnostics = Vec::new();

    for declaration in declarations {
        register_entity_kinds(&mut kind_reg, declaration, &mut diagnostics);
        register_fields(&mut field_reg, declaration, &mut diagnostics);
        collect_edge_types(&mut declared_edges, declaration, &mut diagnostics);
    }

    // After every extension, apply the entity enhancements.
    diagnostics.extend(apply_entity_enhancements(
        declarations,
        &mut kind_reg,
        &mut field_reg,
    ));

    // Once every kind, field and enhancement is registered: each kind's scope field (ADR 0284).
    resolve_scopes(&mut kind_reg, &field_reg, &mut diagnostics);

    // Once every kind, field and enhancement is registered: each edge type
    // with every field that writes it (ADR 0060).
    let (edge_reg, edge_diagnostics) =
        edge_types::resolve(declarations, declared_edges, &field_reg);
    diagnostics.extend(
        edge_diagnostics
            .into_iter()
            .map(|d| Reported::new(BuildPart::Registries, d)),
    );

    (kind_reg, field_reg, edge_reg, diagnostics)
}

/// Check each kind's `scoped_by` (ADR 0284) and keep the ones that hold: the field is a registered single
/// `reference` with a target kind that is a loaded kind other than the kind itself (a structural keyword is no
/// loaded kind), and it closes no scope cycle (the kinds form a DAG). Each failure is W021 and leaves the kind
/// unscoped. Kinds are taken in keyword order, so which kind of a cycle is left unscoped does not depend on the
/// registry's hash order.
fn resolve_scopes(
    kinds: &mut KindRegistry,
    fields: &FieldRegistry,
    diagnostics: &mut Vec<Reported>,
) {
    let mut declared: Vec<(String, String)> = kinds
        .iter()
        .filter_map(|(kind, entry)| Some((kind.clone(), entry.declared.scoped_by.clone()?)))
        .collect();
    declared.sort();
    // The accepted scope kind of each scoped kind.
    let mut parent: BTreeMap<String, String> = BTreeMap::new();
    for (kind, field) in declared {
        let extension = kinds
            .get(&kind)
            .map(|e| e.source_extension.clone())
            .unwrap_or_default();
        let refuse = |why: String| {
            Reported::new(
                BuildPart::Registries,
                Diagnostic::new(
                    codes::W021,
                    format!(
                        "extension '{extension}': kind '{kind}' declares scoped_by '{field}', which {why}"
                    ),
                ),
            )
        };
        if field.contains('.') {
            diagnostics.push(refuse(
                "is a member path: a scope is one kind, named by a field of the kind".into(),
            ));
            continue;
        }
        let Some(entry) = fields.get(&kind, &field) else {
            diagnostics.push(refuse("is not one of its fields".into()));
            continue;
        };
        if entry.field_type() != FieldType::Reference {
            diagnostics.push(refuse("is not a single reference field".into()));
            continue;
        }
        if entry.form().target_kinds().len() > 1 {
            diagnostics.push(refuse(
                "names several kinds with target_kinds: a scope is one kind".into(),
            ));
            continue;
        }
        let Some(target) = entry.form().target_kind() else {
            diagnostics.push(refuse("declares no target kind".into()));
            continue;
        };
        if target == kind {
            diagnostics.push(refuse("targets the kind itself".into()));
            continue;
        }
        if !kinds.contains(target) {
            diagnostics.push(refuse(format!(
                "targets '{target}', which is no loaded entity kind"
            )));
            continue;
        }
        // The scopes above `target` must not lead back to `kind`.
        let mut above = parent.get(target);
        let mut closes = false;
        while let Some(up) = above {
            if *up == kind {
                closes = true;
                break;
            }
            above = parent.get(up);
        }
        if closes {
            diagnostics.push(refuse(format!(
                "targets '{target}', which closes a scope cycle"
            )));
            continue;
        }
        parent.insert(kind.clone(), target.to_string());
        if let Some(entry) = kinds.get_mut(&kind) {
            entry.scoped_by = Some(field);
        }
    }
}

/// Apply entity enhancements to the FieldRegistry, declarations in load
/// order. Enhancement fields do NOT overwrite existing kind-level fields.
///
/// An enhancement whose target kind is unknown is skipped: silently when its
/// extension declares an optional peer the project does not load (the project
/// does not use it), else as I004. Its fields are its extension's (ADR 0096).
///
/// An enhancement carrying `verify_kinds` makes its target kind testable
/// with exactly those kinds (ADR 0002).
fn apply_entity_enhancements(
    declarations: &[ExtensionDeclaration],
    kind_reg: &mut KindRegistry,
    field_reg: &mut FieldRegistry,
) -> Vec<Reported> {
    let mut diagnostics = Vec::new();
    let loaded: Vec<&str> = declarations.iter().map(|d| d.name()).collect();

    for declaration in declarations {
        for enhancement in &declaration.enhancements {
            apply_enhancement(
                declaration,
                enhancement,
                &loaded,
                kind_reg,
                field_reg,
                &mut diagnostics,
            );
        }
    }

    diagnostics
}

/// Whether `declaration` declares an optional peer the project does not load.
fn has_unloaded_optional_peer(declaration: &ExtensionDeclaration, loaded: &[&str]) -> bool {
    declaration
        .peers()
        .iter()
        .any(|p| p.optional && !loaded.contains(&p.name.as_str()))
}

fn apply_enhancement(
    declaration: &ExtensionDeclaration,
    enhancement: &EntityEnhancementDescriptor,
    loaded: &[&str],
    kind_reg: &mut KindRegistry,
    field_reg: &mut FieldRegistry,
    diagnostics: &mut Vec<Reported>,
) {
    if !kind_reg.contains(&enhancement.target_kind) {
        if !has_unloaded_optional_peer(declaration, loaded) {
            diagnostics.push(Reported::new(
                BuildPart::Registries,
                Diagnostic::new(
                    codes::I004,
                    format!(
                        "extension '{}': entity enhancement targets unknown kind '{}' (extension may not be installed)",
                        declaration.name(),
                        enhancement.target_kind
                    ),
                ),
            ));
        }
        return;
    }

    if let Some(verify_kinds) = &enhancement.verify_kinds
        && let Some(kind) = kind_reg.get_mut(&enhancement.target_kind)
    {
        kind.testable = true;
        kind.supports_verify = true;
        kind.allowed_verify_kinds = verify_kinds.clone();
    }

    for field in &enhancement.fields {
        // Kind-level fields win: do not overwrite.
        if field_reg.contains(&enhancement.target_kind, &field.name) {
            continue;
        }
        register_single_field(
            field_reg,
            &enhancement.target_kind,
            field,
            declaration.name(),
            diagnostics,
        );
    }
}

/// Register a declaration's entity kinds into the KindRegistry.
fn register_entity_kinds(
    registry: &mut KindRegistry,
    declaration: &ExtensionDeclaration,
    diagnostics: &mut Vec<Reported>,
) {
    for kind in &declaration.entities {
        let keyword = keyword(kind);
        let entry = KindRegistryEntry {
            kind_name: keyword.to_string(),
            source_extension: declaration.name().to_string(),
            testable: kind.testable,
            supports_verify: kind.supports_verify,
            allowed_verify_kinds: kind.verify_kinds.clone(),
            // Read and ignored since ADR 0287; the build cache that used it goes with ticket 10.
            lifecycle_field: kind.lifecycle_field.clone(),
            scoped_by: None,
            declared: kind.clone(),
        };
        if let Some(existing) = registry.register(entry) {
            // Duplicate — first extension wins (already registered), emit E026
            diagnostics.push(Reported::new(
                BuildPart::Collision,
                Diagnostic::new(
                    codes::E026,
                    format!(
                        "entity kind '{}' registered by '{}' conflicts with '{}' (first registration wins)",
                        keyword,
                        declaration.name(),
                        existing.source_extension
                    ),
                )
                .with_data(DiagnosticData::ShadowedKeyword {
                    keyword: keyword.to_string(),
                }),
            ));
            // Restore the first registration (it wins)
            registry.register(existing);
        }
    }
}

/// Register a declaration's fields into the FieldRegistry: its shared
/// fields on every kind it declares, then each kind's own.
fn register_fields(
    registry: &mut FieldRegistry,
    declaration: &ExtensionDeclaration,
    diagnostics: &mut Vec<Reported>,
) {
    for kind in &declaration.entities {
        // Extension-level shared fields first
        for field in &declaration.shared_fields {
            register_single_field(
                registry,
                keyword(kind),
                field,
                declaration.name(),
                diagnostics,
            );
        }
        // Kind-level fields override extension-level
        for field in &kind.fields {
            register_single_field(
                registry,
                keyword(kind),
                field,
                declaration.name(),
                diagnostics,
            );
        }
    }
}

fn register_single_field(
    registry: &mut FieldRegistry,
    kind_name: &str,
    field: &FieldDescriptor,
    source_extension: &str,
    diagnostics: &mut Vec<Reported>,
) {
    match FieldRegistryEntry::new(kind_name, source_extension, field.clone()) {
        Ok(entry) => {
            // W021 comes before the entry is registered.
            if let Some(role) = field.proof_role.as_deref()
                && entry.proof_role().is_none()
            {
                diagnostics.push(Reported::new(
                    BuildPart::Registries,
                    Diagnostic::new(
                        codes::W021,
                        format!(
                            "extension '{}': field '{}' on kind '{}' declares proof_role '{}': expected 'bound' or 'claim'",
                            source_extension, field.name, kind_name, role
                        ),
                    ),
                ));
            }
            for problem in entry.problems() {
                diagnostics.push(Reported::new(
                    BuildPart::Registries,
                    match problem {
                        FieldProblem::UnknownMemberType { member, field_type } => Diagnostic::new(
                            codes::W019,
                            format!(
                                "extension '{source_extension}': unknown field type '{field_type}' for member '{member}' on kind '{kind_name}'"
                            ),
                        ),
                        FieldProblem::Refused { field, what } => Diagnostic::new(
                            codes::W021,
                            format!(
                                "extension '{source_extension}': field '{field}' on kind '{kind_name}' {what}"
                            ),
                        ),
                    },
                ));
            }
            registry.register(entry);
        }
        Err(UnknownFieldType(name)) => diagnostics.push(Reported::new(
            BuildPart::Registries,
            Diagnostic::new(
                codes::W019,
                format!(
                    "extension '{}': unknown field type '{}' for field '{}' on kind '{}'",
                    source_extension, name, field.name, kind_name
                ),
            ),
        )),
    }
}

/// Collect a declaration's explicit edge types into `declared` (the
/// declaring extension with each): the first declaration of a label wins, a
/// later one is W018.
fn collect_edge_types(
    declared: &mut Vec<(String, EdgeTypeDescriptor)>,
    declaration: &ExtensionDeclaration,
    diagnostics: &mut Vec<Reported>,
) {
    for edge in &declaration.edges {
        match declared.iter().find(|(_, first)| first.label == edge.label) {
            Some((first_extension, first)) => diagnostics.push(Reported::new(
                BuildPart::Collision,
                Diagnostic::new(
                    codes::W018,
                    format!(
                        "edge type '{}' from '{}' duplicates '{}' from '{}' (first wins)",
                        edge.label,
                        declaration.name(),
                        first.label,
                        first_extension
                    ),
                ),
            )),
            None => declared.push((declaration.name().to_string(), edge.clone())),
        }
    }
}
