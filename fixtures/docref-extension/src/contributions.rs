//! What the docref extension declares: a kind whose `docs` field names
//! files (`file_reference`) and a `file_exists` rule over its `guide` field.
//! The host's session tests load it to exercise the files the checks read.
//!
//! The vendored release blob `docref.wasm` is built from this source;
//! rebuild with `cargo build --release --target wasm32-wasip2` and copy
//! `target/wasm32-wasip2/release/docref.wasm` next to this crate.

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
        c.kind("gadget", |k| {
            k.description("A thing with docs").testable(false);
            k.field("docs", |f| {
                f.field_type(FieldType::StringList).file_reference();
            });
            k.field("guide", |f| {
                f.field_type(FieldType::String);
            });
        });
        c.rule("W901", |r| {
            r.check(CheckKind::FileExists)
                .severity(ValidationSeverity::Warning)
                .target_kind("gadget")
                .field("guide")
                .message_template("gadget '{id}': missing '{value}'");
        });
    }
}

pub fn build() -> ContributionsBuilder {
    specforge_extension_build()
}
