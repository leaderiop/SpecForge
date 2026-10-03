//! Registries for trace tests: what two made-up extensions declare.

use specforge_registry::{
    FieldRegistry, FieldRegistryEntry, KindRegistry, KindRegistryEntry, ManifestFieldType,
};

fn kind(name: &str, extension: &str) -> KindRegistryEntry {
    KindRegistryEntry {
        kind_name: name.to_string(),
        description: None,
        source_extension: extension.to_string(),
        testable: false,
        singleton: false,
        supports_verify: false,
        allowed_verify_kinds: Vec::new(),
        has_body_parser: false,
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        open_fields: false,
        contract_target: false,
        declares_types: false,
        lifecycle_field: None,
    }
}

/// A reference-list field `kind.name -> target`, declared by `extension`.
pub fn reference(
    kind: &str,
    name: &str,
    target: &str,
    extension: &str,
    inverse_of: Option<&str>,
) -> FieldRegistryEntry {
    FieldRegistryEntry {
        kind_name: kind.to_string(),
        field_name: name.to_string(),
        description: None,
        field_type: ManifestFieldType::ReferenceList,
        source_extension: extension.to_string(),
        edge: Some(format!("{kind}_{name}")),
        target_kind: Some(target.to_string()),
        file_reference: false,
        required: false,
        inverse_of: inverse_of.map(str::to_string),
        normative: false,
        exempts_obligations: false,
        headline: false,
        derived_from: None,
        proof_role: None,
    }
}

/// `@t/soft` declares behavior and invariant, `@t/prod` declares feature,
/// `@t/formal` adds a field to behavior. Of behavior's reference fields
/// only `invariants` and `features` are expected edges: `ports` points at a
/// kind nothing loads, `depends_on` at behavior itself, `satisfies` comes
/// from another extension, and `contract` is a string, not a reference.
/// `feature.behaviors` is expected of features.
pub fn registries() -> (FieldRegistry, KindRegistry) {
    let mut kinds = KindRegistry::new();
    kinds.register(kind("behavior", "@t/soft"));
    kinds.register(kind("invariant", "@t/soft"));
    kinds.register(kind("feature", "@t/prod"));

    let mut fields = FieldRegistry::new();
    fields.register(reference(
        "behavior",
        "invariants",
        "invariant",
        "@t/soft",
        Some("enforced_by"),
    ));
    fields.register(reference(
        "behavior",
        "features",
        "feature",
        "@t/soft",
        Some("behaviors"),
    ));
    fields.register(reference("behavior", "ports", "port", "@t/soft", None));
    fields.register(reference(
        "behavior",
        "depends_on",
        "behavior",
        "@t/soft",
        None,
    ));
    fields.register(reference(
        "behavior",
        "satisfies",
        "invariant",
        "@t/formal",
        None,
    ));
    let mut contract = reference("behavior", "contract", "invariant", "@t/soft", None);
    contract.field_type = ManifestFieldType::String;
    fields.register(contract);
    fields.register(reference(
        "feature",
        "behaviors",
        "behavior",
        "@t/prod",
        None,
    ));
    (fields, kinds)
}
