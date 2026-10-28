//! The SDK declares a file read and `reads_tests` (protocol 1.11.0, ADR 0293) and a command reads its `files`.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::testing::{CommandFeed, call_every_command, call_every_command_with};
use specforge_protocol_types::{FileFact, FileFacts};

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

#[test]
fn a_command_reads_the_hosts_clock_as_now() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/clock", "1.0.0"));
    c.command("stamp", |cmd| {
        cmd.title("Stamp")
            .description("Stamp")
            .handler(|call: &CommandCall| {
                CommandOutput::ok(format!("{} {}", call.today(), call.now()))
            });
    });
    let out = call_every_command(&c, &CommandGraph::default(), "2026-10-28", |_, _| {
        String::new()
    });
    assert_eq!(out[0].1.stdout, "2026-10-28 2026-10-28T00:00:00Z");
}

#[test]
fn the_harness_feeds_a_command_its_files_and_an_explicit_now() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/feed", "1.0.0"));
    c.command("count", |cmd| {
        cmd.title("Count")
            .description("Count")
            .reads("src", FileReadSpec::glob("src/*.rs").text())
            .handler(|call: &CommandCall| {
                let facts = call
                    .files("src")
                    .map(|f| f.facts.as_slice())
                    .unwrap_or_default();
                let text = facts
                    .iter()
                    .filter_map(|f| f.text.as_deref())
                    .collect::<String>();
                CommandOutput::ok(format!("{} {text} {}", facts.len(), call.now()))
            });
    });
    let feed = CommandFeed {
        now: Some("2026-10-28T23:59:59Z".into()),
        files: [(
            "src".to_string(),
            FileFacts {
                facts: vec![FileFact {
                    path: "src/a.rs".into(),
                    exists: true,
                    text: Some("fn a() {}".into()),
                    ..FileFact::default()
                }],
                truncated: false,
            },
        )]
        .into(),
    };

    let out = call_every_command_with(&c, &CommandGraph::default(), "2026-10-28", &feed, |_, _| {
        String::new()
    });
    assert_eq!(out[0].1.stdout, "1 fn a() {} 2026-10-28T23:59:59Z");

    // Without a feed the command sees no file, as before.
    let bare = call_every_command(&c, &CommandGraph::default(), "2026-10-28", |_, _| {
        String::new()
    });
    assert_eq!(bare[0].1.stdout, "0  2026-10-28T00:00:00Z");
}
