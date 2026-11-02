//! A collector's declared reads (ADR 0293): collect reads them from the plain disk after the run and hands their
//! facts to the export as `files`, held to the project as a pass's are.

use serde_json::Value;
use specforge_ops::testing::ScriptedMachine;
use specforge_protocol_types::FileRead;
use specforge_test_macros::test as specforge_test;

use super::{Fixture, Runner};

/// A glob read of `glob`, with the files' text when `text`.
fn glob(glob: &str, text: bool) -> FileRead {
    FileRead {
        glob: Some(glob.to_string()),
        text,
        ..Default::default()
    }
}

/// Collect `acme` (declaring `reads`) from its existing report, with `files` written first: what its export received.
fn collected(reads: &[(&str, FileRead)], files: &[(&str, &str)]) -> Value {
    let runner = reads
        .iter()
        .fold(Runner::new("acme"), |runner, (name, read)| {
            runner.reading(name, read.clone())
        });
    let fixture = Fixture::new(vec![runner]);
    fixture.write("out/acme.json", "{}");
    for (path, text) in files {
        fixture.write(path, text);
    }
    fixture
        .no_run(Some("acme"), &ScriptedMachine::new())
        .expect("the collect runs");
    let calls = fixture.dispatched();
    calls
        .iter()
        .find(|call| call.export == "collect__acme")
        .expect("the collector was dispatched")
        .input
        .clone()
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a collector receives the facts of the files it declares it reads, and none when it declares none"
)]
fn a_collector_receives_the_facts_of_the_files_it_reads() {
    let input = collected(
        &[
            ("manifest", glob("package.json", true)),
            ("facts", glob("package.json", false)),
        ],
        &[("package.json", "{}\n")],
    );

    let manifest = &input["files"]["manifest"]["facts"][0];
    assert_eq!(manifest["path"], "package.json");
    assert_eq!(manifest["exists"], true);
    assert!(
        manifest["digest"].as_str().unwrap().starts_with("sha256:"),
        "{manifest}"
    );
    assert_eq!(manifest["text"], "{}\n");
    assert!(input["files"]["facts"]["facts"][0].get("text").is_none());
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a collector receives the facts of the files it declares it reads, and none when it declares none"
)]
fn a_collector_that_declares_no_reads_is_sent_no_files() {
    let input = collected(&[], &[("package.json", "{}")]);

    assert!(input.get("files").is_none(), "{input}");
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a collector's read is held to the project and withholds a credential's text, as a pass's is"
)]
fn a_collectors_read_withholds_a_credentials_text() {
    let input = collected(&[("env", glob(".env", true))], &[(".env", "TOKEN=x\n")]);

    let fact = &input["files"]["env"]["facts"][0];
    assert_eq!(fact["error"], "denied", "{fact}");
    assert!(fact.get("text").is_none(), "{fact}");
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a collector's read is held to the project and withholds a credential's text, as a pass's is"
)]
fn a_collectors_read_outside_the_project_is_refused() {
    let input = collected(&[("outside", glob("../x", false))], &[]);
    assert!(input["files"].get("outside").is_none(), "{input}");

    #[cfg(unix)]
    {
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::write(elsewhere.path().join("secret.json"), "{}").unwrap();
        let runner = Runner::new("acme").reading("linked", glob("linked.json", false));
        let fixture = Fixture::new(vec![runner]);
        fixture.write("out/acme.json", "{}");
        std::os::unix::fs::symlink(
            elsewhere.path().join("secret.json"),
            fixture.root().join("linked.json"),
        )
        .unwrap();
        fixture
            .no_run(Some("acme"), &ScriptedMachine::new())
            .expect("the collect runs");
        let calls = fixture.dispatched();
        let input = &calls
            .iter()
            .find(|call| call.export == "collect__acme")
            .unwrap()
            .input;
        assert_eq!(
            input["files"]["linked"]["facts"][0]["error"], "outside_project",
            "{input}"
        );
    }
}
