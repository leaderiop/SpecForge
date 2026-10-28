//! The declarative rules of `@gsd/core`. `gsd/E107` is declared with the kinds (`kinds::dsl`), over each authored
//! `ts` field and block member (none yet: `kinds::dsl`'s tests declare them on test kinds); the rest are here.

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    // E105: an `out_of_scope` requirement has a reason.
    c.rule("E105", |r| {
        r.check(CheckKind::ConditionalFieldRequired)
            .target_kind("requirement")
            .field("reason")
            .severity(ValidationSeverity::Error)
            .message_template("requirement '{id}' is out of scope but gives no reason");
        r.constraint(|fc| {
            fc.kind(ConstraintKind::WhenFieldEquals)
                .pattern("tier")
                .values(&["out_of_scope"]);
        });
    });

    // E201: a context has canonical refs. The field is not `required` (that would be the host's own error), so this
    // code is the one diagnostic of its absence. An unwritten field breaks no `non_empty`, a written `[]` is not
    // missing: each case fires exactly one of the two rules.
    c.rule("E201", |r| {
        r.check(CheckKind::MissingRequiredField)
            .target_kind("context")
            .field("canonical_refs")
            .severity(ValidationSeverity::Error)
            .message_template("context '{id}' has no canonical_refs; with no external spec, write one whose topic says so");
    });
    c.rule("E201", |r| {
        r.check(CheckKind::FieldValueConstraint)
            .target_kind("context")
            .field("canonical_refs")
            .constraint(|fc| {
                fc.kind(ConstraintKind::NonEmpty);
            })
            .severity(ValidationSeverity::Error)
            .message_template("context '{id}' has an empty canonical_refs");
    });
}
