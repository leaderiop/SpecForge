//! The SDK declares a file read and `reads_tests` (protocol 1.11.0, ADR 0293) and a command reads its `files`.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_extension_sdk::prelude::*;

#[test]
fn the_sdk_declares_reads_and_reads_tests() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/reads", "1.0.0"));
    c.pass("audit", |p| {
        p.phase("check")
            .reads(
                "modified",
                FileReadSpec::field("plan.files_modified").text(),
            )
            .reads(
                "sources",
                FileReadSpec::glob("src/**/*.rs").base(FileBase::Spec),
            )
            .reads_tests()
            .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
    });
    c.command("verify", |cmd| {
        cmd.title("Verify")
            .description("Verify")
            .reads("plans", FileReadSpec::field("plan.files_modified"))
            .handler(|call: &CommandCall| {
                let seen = call.files("plans").map_or(0, |f| f.facts.len());
                CommandOutput::ok(format!("{seen}"))
            });
    });
    let declaration = c.declaration();

    let pass = &declaration.passes[0];
    assert!(pass.reads_tests);
    assert_eq!(pass.reads.len(), 2);
    assert_eq!(pass.reads[0].name, "modified");
    assert_eq!(pass.reads[0].field.as_deref(), Some("plan.files_modified"));
    assert!(pass.reads[0].text);
    assert_eq!(pass.reads[1].glob.as_deref(), Some("src/**/*.rs"));
    assert_eq!(pass.reads[1].base.as_deref(), Some("spec"));
    let command = &declaration.surfaces.commands[0];
    assert_eq!(command.reads[0].name, "plans");
}
