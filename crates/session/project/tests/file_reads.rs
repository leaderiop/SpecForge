//! K11 (ADR 0293): a pass declares the project files it reads and its input carries their facts and capped text; a
//! read file is a check input. Pins for the ticket that delivers them; each is ignored until then.

use std::fs;
use std::sync::Arc;

use serde_json::json;
use specforge_project::testing::{LIB_RS, gsd_facts_project, gsd_facts_runtime, pass_input};
use specforge_project::{SharedRuntime, UpdateKind};

#[test]
#[ignore = "pins K11; T5 un-ignores"]
fn a_pass_reads_a_declared_file_fact_and_text() {
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_runtime());
    let _compiled = project.compile_in(runtime.clone());

    let input = pass_input(&runtime, "files");
    let facts = &input["files"]["modified"]["facts"];
    let lib = facts
        .as_array()
        .and_then(|all| all.iter().find(|f| f["path"] == "src/lib.rs"))
        .unwrap_or_else(|| panic!("no fact for src/lib.rs: {input}"));
    assert_eq!(lib["exists"], true);
    assert_eq!(lib["entity"], "pl1");
    assert_eq!(lib["bytes"], LIB_RS.len());
    assert_eq!(lib["lines"], 3);
    assert_eq!(lib["text"], LIB_RS);
    assert!(
        lib["digest"]
            .as_str()
            .is_some_and(|d| d.starts_with("sha256:")),
        "{lib}"
    );
    let gone = facts
        .as_array()
        .and_then(|all| all.iter().find(|f| f["path"] == "docs/gone.md"))
        .unwrap_or_else(|| panic!("no fact for docs/gone.md: {input}"));
    assert_eq!(gone["exists"], false);
    assert!(gone.get("text").is_none(), "{gone}");
    // The glob read gives facts and no text.
    let sources = &input["files"]["sources"]["facts"];
    assert_eq!(sources[0]["path"], "src/lib.rs", "{input}");
    assert!(sources[0].get("text").is_none(), "{input}");
}

#[test]
#[ignore = "pins K11; T5 un-ignores"]
fn editing_a_read_file_reruns_the_pass() {
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_runtime());
    let mut session = project.session_in(runtime.clone() as SharedRuntime);
    runtime.clear_calls();

    let lib = project.root().join("src/lib.rs");
    fs::write(&lib, "pub fn two() -> u8 {\n    2\n}\n").unwrap();
    let update = session
        .apply(&session.changes([lib.as_path()]))
        .expect("a file a pass reads is a check input");
    assert_eq!(update.kind, UpdateKind::Checks);

    let input = pass_input(&runtime, "files");
    assert_eq!(
        input["files"]["modified"]["facts"][1]["text"],
        json!("pub fn two() -> u8 {\n    2\n}\n"),
        "{input}"
    );
}
