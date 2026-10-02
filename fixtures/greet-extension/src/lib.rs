//! The greet extension, authored entirely with the SpecForge extension SDK.
//! Sixteen lines of intent; the SDK generates every protocol export.

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@sdk/greet",
    version = "0.1.0",
    short = "Friendly greetings"
)]
struct Greet;

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
                fc.kind("matches");
                fc.pattern("^(warm|formal)$");
            });
            r.severity(ValidationSeverity::Error);
            r.message_template("greeting '{id}' has unknown style");
        });
    }
}

fn dispatch(_export: &str, _input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    None
}

specforge_extension_sdk::component_guest!(
    build = specforge_extension_build,
    handler = dispatch
);
