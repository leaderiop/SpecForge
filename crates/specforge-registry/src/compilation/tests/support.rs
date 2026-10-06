//! Declarations the registry's engine tests build from, with the SDK's
//! builders, as an extension declares itself, and the one way they reach
//! the registries: `registries`, which is `build_registries`.

use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::ExtensionDeclaration;

/// A builder for the extension `name`, version 1.0.0.
pub(crate) fn extension(name: &str) -> ContributionsBuilder {
    ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"))
}

/// The declaration of an extension `name` (version 1.0.0) that `f` fills.
pub(crate) fn declare(
    name: &str,
    f: impl FnOnce(&mut ContributionsBuilder),
) -> ExtensionDeclaration {
    let mut c = extension(name);
    f(&mut c);
    c.declaration()
}

/// `@specforge/software`: the testable `behavior` (enforcing `invariant`s
/// over the `enforces` edge) and `invariant` kinds.
pub(crate) fn software() -> ExtensionDeclaration {
    declare("@specforge/software", |c| {
        c.kind("Behavior", |k| {
            k.keyword("behavior")
                .testable(true)
                .singleton(false)
                .supports_verify(true)
                .verify_kinds(&["unit", "contract", "integration"])
                .semantic_token("function")
                .lsp_icon("Method")
                .dot_shape("ellipse");
            k.field("contract", |f| {
                f.field_type(FieldType::Block);
            });
            k.field("invariants", |f| {
                f.field_type(FieldType::ReferenceList)
                    .edge("enforces")
                    .target_kind("invariant");
            });
        });
        c.kind("Invariant", |k| {
            k.keyword("invariant")
                .testable(true)
                .supports_verify(true)
                .dot_shape("diamond");
        });
        c.edge("enforces", |e| {
            e.source_kind("behavior")
                .target_kind("invariant")
                .edge_style("dashed");
        });
    })
}

/// The registry build of `declarations`, in this load order.
pub(crate) fn registries(declarations: &[ExtensionDeclaration]) -> crate::RegistryBuild {
    crate::build_registries(declarations.to_vec())
}
