//! The declarative rules of `@gsd/core`. `gsd/E107` is declared with the kinds (`kinds::dsl`), over each authored
//! `ts` field (none in r34; `kinds::dsl`'s tests declare one on a test kind); the rest are here.

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
}
