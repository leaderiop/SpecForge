//! PROTOTYPE — the entire greet extension, in the proposed SDK style.
//! Compare with extensions/*/src/lib.rs today: ~40 lines of handshake
//! boilerplate + 9 hand-maintained describe JSON files.

use specforge_extension_sdk::prelude::*;

#[specforge::extension(name = "greet", version = "0.1.0", short = "Friendly greetings")]
struct Greet;

impl Contributions for Greet {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind("greeting", |k| {
            k.title("Greeting").testable(false).field("style", |f| {
                f.kind(FieldType::Enum).values(["warm", "formal"]).required()
            });
        });
        c.rule("greeting_style_known", |r| {
            r.check(Check::FieldValue { field: "style", matches: "^(warm|formal)$" })
                .severity(Severity::Error)
                .message("unknown greeting style")
        });
    }
}
