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

    // E307: each checkpoint kind has its fields (`src/verify.cts:1206-1232`). `resume_signal` is required by the
    // kind itself. The options of a decision and `auto_select` are checked by the plan-shape pass.
    for (kind, field) in [
        ("human_verify", "what_built"),
        ("human_verify", "how_to_verify"),
        ("decision", "decision"),
        ("decision", "options"),
        ("human_action", "instructions"),
        ("human_action", "action"),
        ("human_action", "verification"),
    ] {
        c.rule("E307", |r| {
            r.check(CheckKind::ConditionalFieldRequired)
                .target_kind("checkpoint")
                .field(field)
                .severity(ValidationSeverity::Error)
                .message_template(&format!(
                    "checkpoint '{{id}}' of kind {kind} has no {field}"
                ));
            r.constraint(|fc| {
                fc.kind(ConstraintKind::WhenFieldEquals)
                    .pattern("kind")
                    .values(&[kind]);
            });
        });
    }

    // E319: a prohibition checked by a test names it (the prohibition probe's LOCATE guard fails closed without a
    // check target).
    c.rule("E319", |r| {
        r.check(CheckKind::ConditionalFieldRequired)
            .target_kind("prohibition")
            .field("test")
            .severity(ValidationSeverity::Error)
            .message_template("prohibition '{id}' is verified by a test but names none");
        r.constraint(|fc| {
            fc.kind(ConstraintKind::WhenFieldEquals)
                .pattern("verification")
                .values(&["test"]);
        });
    });
}
