//! A command that declares `reads` is sent the facts of its files (ADR 0293); one that declares none is sent no
//! `files`.

use super::*;

fn reading(reads: Value) -> ExtensionCommand {
    declared(json!({
        "id": "reading", "title": "Reading", "description": "", "export": "cmd__ok", "args": [],
        "reads": reads,
    }))
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "the CommandInput carries the format the caller asked for and the host's date"
)]
fn a_command_with_reads_gets_files_and_one_without_gets_none() {
    let runtime = std::sync::Arc::new(fake());
    let fixture = fixture(&runtime);
    std::fs::write(fixture.root().join("a.txt"), "one\ntwo").unwrap();
    std::fs::write(fixture.root().join("b.md"), "# b\n").unwrap();

    run_on(
        &fixture.view(),
        &reading(json!([{"name": "docs", "glob": "*.txt", "text": true}])),
        &Map::new(),
        CommandFormat::Json,
        at("2026-10-03"),
    )
    .unwrap();
    let input = last_input(&runtime);
    let facts = &input["files"]["docs"]["facts"];
    assert_eq!(facts.as_array().unwrap().len(), 1, "{input}");
    assert_eq!(facts[0]["path"], "a.txt");
    assert_eq!(facts[0]["text"], "one\ntwo");
    assert_eq!(facts[0]["lines"], 2);
    assert!(facts[0]["digest"].as_str().unwrap().starts_with("sha256:"));

    run_on(
        &fixture.view(),
        &routed("ok"),
        &Map::new(),
        CommandFormat::Json,
        at("2026-10-03"),
    )
    .unwrap();
    assert!(last_input(&runtime).get("files").is_none());
}
