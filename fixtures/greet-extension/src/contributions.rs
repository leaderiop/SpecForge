//! What the greet extension declares, authored entirely with the SpecForge
//! extension SDK. Kept apart from the component glue (`lib.rs`) so the
//! host's tests can serve the same declarations in process
//! (`crates/specforge-component/tests/greet_sdk.rs` includes this file) and
//! check both runtimes answer alike.

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@sdk/greet",
    version = "0.1.0",
    short = "greet",
    description = "Friendly greetings"
)]
pub struct Greet;

impl Contributions for Greet {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind("greeting", |k| {
            k.description("A friendly greeting").testable(false);
            k.field("style", |f| {
                f.field_type(FieldType::Enum);
                f.enum_values(&["warm", "formal"]);
                f.required();
            });
        });
        c.rule("G101", |r| {
            r.check(CheckKind::FieldValueConstraint);
            r.target_kind("greeting");
            r.field("style");
            r.constraint(|fc| {
                fc.kind(ConstraintKind::Matches);
                fc.pattern("^(warm|formal)$");
            });
            r.severity(ValidationSeverity::Error);
            r.message_template("greeting '{id}' has unknown style");
        });
    }
}

/// The extension's contributions, as its guest builds them per call.
pub fn build() -> ContributionsBuilder {
    specforge_extension_build()
}
