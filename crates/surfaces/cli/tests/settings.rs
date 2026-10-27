//! `specforge settings` (ADR 0288).

use crate::support::{Cli, TestProject};

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "settings lists each setting's type, value, origin, default, gates, raises and each secret's status"
)]
fn settings_lists_value_origin_gates_raises_and_secrets() {
    // The builtins declare no setting: a project that sets one is told so (the typed listing is tested over the
    // view in `specforge-ops`, where an extension is served in process).
    let project = TestProject::new()
        .raw_config(r#"{"name": "p", "version": "0.1.0", "extensions": [], "settings": {"@x/y": {"a": 1}}}"#)
        .file("a.spec", "");
    let cli = Cli::on(&project);
    let document = cli.document(&["settings", "--all"]);
    assert_eq!(document.code, 0, "{}", document.stderr);
    assert_eq!(document.json["extensions"], serde_json::json!([]));
    assert_eq!(document.json["count"], 0);
    assert_eq!(document.json["diagnostics"][0]["code"], "W166");
    let human = cli.run(&["settings", "--extension", "@x/y"]);
    assert_eq!(human.code, 0);
    assert!(human.stdout.contains("No settings."), "{}", human.stdout);
    assert!(human.stderr.contains("W166"), "{}", human.stderr);
}
