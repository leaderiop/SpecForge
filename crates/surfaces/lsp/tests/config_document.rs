//! The language server serves `specforge.json`'s settings as they are typed (ADR 0288).

use serde_json::json;
use specforge_lsp::changes::Change;
use specforge_lsp::config_document;
use specforge_project::testing::{follower_extension, settings_extension};
use tower_lsp::lsp_types::{
    CompletionResponse, DiagnosticSeverity, FileChangeType, FileEvent, Position, Url,
};

use crate::recorder::publications;
use crate::served::Served;

/// A project serving `@t/s` and `@t/f`, with `text` open as its specforge.json at the `|` in it.
fn editing(text: &str) -> (Served, Url, Position) {
    let marked = text.find('|').expect("the cursor is marked with |");
    let (before, after) = (&text[..marked], &text[marked + 1..]);
    let line = before.matches('\n').count() as u32;
    let character = before.rsplit('\n').next().unwrap().chars().count() as u32;
    let mut served = Served::new(&[("a.spec", "gadget w \"W\" {}\n")])
        .extension("@t/s", |c| *c = settings_extension())
        .extension("@t/f", |c| *c = follower_extension())
        .open(&["a.spec"]);
    let uri = served.uri("specforge.json");
    served.open_document(&uri, &format!("{before}{after}"));
    (served, uri, Position::new(line, character))
}

fn labels(served: &Served, uri: &Url, at: Position) -> Vec<String> {
    let state = served.state();
    match specforge_lsp::answers::completion(&state, uri, at) {
        Some(CompletionResponse::Array(items)) => items.into_iter().map(|i| i.label).collect(),
        other => panic!("{other:?}"),
    }
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "completion in specforge.json offers extension names, unwritten setting names and values"
)]
fn config_completion_offers_settings_keys_and_values() {
    // Extension names at the first level; the setting names not yet written (a snippet with the default) and a
    // family's closed keys at the second; enum values and true/false at a value.
    let (served, uri, at) =
        editing("{\n  \"extensions\": [\"@t/s\", \"@t/f\"],\n  \"settings\": {\n    |\n  }\n}\n");
    assert_eq!(
        labels(&served, &uri, at),
        ["\"@t/s\""],
        "only an extension that declares a setting"
    );

    let (served, uri, at) = editing(
        "{\n  \"settings\": {\n    \"@t/s\": {\n      \"flag\": true,\n      |\n    }\n  }\n}\n",
    );
    let names = labels(&served, &uri, at);
    for listed in [
        "level",
        "mode",
        "notes",
        "profile",
        "models.planning",
        "models.execution",
        "sections",
    ] {
        assert!(names.contains(&listed.to_string()), "{listed} in {names:?}");
    }
    assert!(
        !names.contains(&"flag".to_string()),
        "written already: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n.starts_with("tiers")),
        "a family that takes any key: {names:?}"
    );
    let state = served.state();
    let Some(CompletionResponse::Array(items)) =
        specforge_lsp::answers::completion(&state, &uri, at)
    else {
        panic!("items")
    };
    let level = items.iter().find(|i| i.label == "level").unwrap();
    assert_eq!(level.insert_text.as_deref(), Some("\"level\": 1"));
    drop(state);

    let (served, uri, at) =
        editing("{\n  \"settings\": {\n    \"@t/s\": {\n      \"profile\": \"|\"\n    }\n  }\n}\n");
    assert_eq!(labels(&served, &uri, at), ["\"compat\"", "\"strict\""]);
    let (served, uri, at) =
        editing("{\n  \"settings\": {\n    \"@t/s\": {\n      \"flag\": tr|ue\n    }\n  }\n}\n");
    assert_eq!(labels(&served, &uri, at), ["true", "false"]);
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "hover on a setting key shows its type, default, effective value, origin and gated codes"
)]
fn config_hover_shows_value_and_origin() {
    // explicit | preset (profile = strict) | default; gated codes and their state.
    let text = "{\n  \"settings\": {\n    \"@t/s\": {\n      \"pro|file\": \"strict\",\n      \"level\": 2\n    }\n  }\n}\n";
    let (served, uri, at) = editing(text);
    let state = served.state();
    let hover = |position| match specforge_lsp::answers::hover(&state, &uri, position)
        .unwrap()
        .contents
    {
        tower_lsp::lsp_types::HoverContents::Markup(m) => m.value,
        other => panic!("{other:?}"),
    };
    let profile = hover(at);
    assert!(
        profile.contains("**profile** `enum`") && profile.contains("(written in specforge.json)"),
        "{profile}"
    );
    assert!(profile.contains("default: `\"compat\"`"), "{profile}");
    // `flag` is not written: hovering the extension names its settings.
    let level = hover(Position::new(4, 9));
    assert!(level.contains("value: `2`"), "{level}");
    let extension = hover(Position::new(2, 6));
    assert!(extension.starts_with("**@t/s**"), "{extension}");
    // Not on a key: nothing.
    assert!(specforge_lsp::answers::hover(&state, &uri, Position::new(0, 0)).is_none());
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "specforge.json diagnostics follow the unsaved buffer and a save reloads the environment"
)]
fn config_diagnostics_follow_the_buffer_and_reload_on_save() {
    // E086 at its range on an unsaved edit; cleared on fix; a save reloads (a raise is visible in the .spec diagnostics).
    let typo = "{\n  \"settings\": {\n    \"@t/s\": {\n      \"levl\": 2\n    }\n  }\n}\n";
    let (mut served, uri, _) = editing(&format!("{typo}|"));
    assert!(config_document::is_config(&uri));
    let sent = served.sent();
    let shown = publications(&sent, &uri);
    let last = shown.last().expect("the buffer is checked when it opens");
    assert_eq!(last.len(), 1, "{last:?}");
    assert_eq!(last[0].severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(last[0].range.start, Position::new(3, 6));
    assert!(
        last[0]
            .message
            .contains("'levl' is not a setting of '@t/s'"),
        "{}",
        last[0].message
    );

    // Fixed in the buffer, not saved: cleared.
    served.type_into(&uri, &typo.replace("levl", "level"));
    served.apply(Change::Edited(vec![uri.clone()]));
    let sent = served.sent();
    assert_eq!(publications(&sent, &uri).last().unwrap().len(), 0);
    // It never reached the project as a source.
    assert!(served.state().graph().node("w").is_some());

    // A save: the disk text is the strict profile, the environment reloads, and the raise shows.
    let saved = json!({"name": "t", "version": "0.1.0", "extensions": ["@t/s", "@t/f"], "settings": {"@t/s": {"profile": "strict"}}});
    served.write("specforge.json", &saved.to_string());
    let event = FileEvent {
        uri: uri.clone(),
        typ: FileChangeType::CHANGED,
    };
    let applied = served
        .apply(Change::Watched(vec![event]))
        .expect("specforge.json is an input");
    assert!(applied.environment);
    let sent = served.sent();
    let raised = ["a.spec", "specforge.json"].iter().any(|file| {
        publications(&sent, &served.uri(file)).iter().flatten().any(|d| {
            matches!(&d.code, Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c.ends_with("/W902"))
                && d.severity == Some(DiagnosticSeverity::ERROR)
        })
    });
    assert!(raised, "{sent:#?}");
}
