//! What the docref extension declares: a kind whose `docs` and `guide` fields
//! name files, each checked by `@specforge/checks:file` from the spec root.
//! The host's session tests load it to exercise the files the checks read.
//!
//! The vendored release blob `wasm/docref.wasm` is built from this source;
//! refresh it with `cargo run -p xtask --bin revendor`.

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@sdk/docref",
    version = "0.1.0",
    short = "docref",
    description = "A kind whose docs field names files"
)]
pub struct DocRef;

impl Contributions for DocRef {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@specforge/checks", "^1.0");
        c.kind("gadget", |k| {
            k.description("A thing with docs").testable(false);
            k.field("docs", |f| {
                f.field_type(FieldType::StringList).file(Root::Spec);
            });
            k.field("guide", |f| {
                f.field_type(FieldType::String).file(Root::Spec);
            });
        });
    }
}

pub fn build() -> ContributionsBuilder {
    specforge_extension_build()
}
