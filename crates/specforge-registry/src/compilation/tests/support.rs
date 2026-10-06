//! Declarations the registry's tests build from, with the SDK's builders,
//! as an extension declares itself.

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

/// A required peer dependency on `name` in `version`.
pub(crate) fn peer(name: &str, version: &str) -> PeerDependency {
    PeerDependency {
        name: name.to_string(),
        version: version.to_string(),
        optional: false,
    }
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

/// `@specforge/product`, a peer of `@specforge/software`: the untestable
/// `feature` kind, composing `behavior`s over the `composes` edge.
pub(crate) fn product() -> ExtensionDeclaration {
    let mut c = extension("@specforge/product");
    c.meta
        .peer_dependencies
        .push(peer("@specforge/software", ">=1.0.0"));
    c.kind("Feature", |k| {
        k.keyword("feature").testable(false).dot_shape("box");
        k.field("behaviors", |f| {
            f.field_type(FieldType::ReferenceList)
                .edge("composes")
                .target_kind("behavior");
        });
    });
    c.edge("composes", |e| {
        e.source_kind("feature").target_kind("behavior");
    });
    c.declaration()
}

/// The kind collisions (E026) populating the registries from
/// `declarations`, in load order, reports.
pub(crate) fn kind_collisions(
    declarations: &[ExtensionDeclaration],
) -> Vec<specforge_common::Diagnostic> {
    crate::compilation::populate::populate(declarations)
        .3
        .into_iter()
        .filter(|d| d.code == "E026")
        .collect()
}
