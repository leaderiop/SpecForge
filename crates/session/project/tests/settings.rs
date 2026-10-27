//! A project that sets no setting reads, calls and reports as it did before ADR 0288: no settings
//! diagnostic, and no call input holds a `settings` key.

use std::sync::Arc;

use specforge_extension_sdk::prelude::*;
use specforge_project::testing::TestProject;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@pin/plain";

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.kind("gadget", |k| {
        k.keyword("gadget");
    });
    c.pass("audit", |p| {
        p.phase("check").run(|_: &PassInput| Vec::new());
    });
    c
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a project without settings reads and calls as it did before settings existed"
)]
fn a_project_without_settings_reads_and_calls_as_today() {
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    let compiled = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", "gadget a \"A\" {}\n")
        .compile_in(runtime.clone());
    for d in compiled.diagnostics() {
        assert!(
            !matches!(d.code.as_str(), "E086" | "E087" | "W166"),
            "{d:?}"
        );
    }
    let calls = runtime.calls();
    assert!(!calls.is_empty());
    for call in calls {
        assert!(call.input.get("settings").is_none(), "{}", call.export);
    }
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a setting resolves explicit, else preset, else default, else unset, and records where it came from"
)]
#[ignore = "K6 T5"]
fn settings_resolve_explicit_over_preset_over_default() {
    // Origin of each; an explicit value E086 refuses falls back to preset or default; `null` unsets a setting with no default.
    unimplemented!("K6 T5: ResolvedSettings::resolve");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a preset that follows a peer extension's setting is inert while the peer is not loaded"
)]
#[ignore = "K6 T5"]
fn a_follower_preset_reads_its_peers_setting() {
    // `@t/f` raises W904 under `@t/s` strict; inert when `@t/s` is absent.
    unimplemented!("K6 T5: peer preset");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a setting not used as written is E086 at its key with the line, the column and a suggestion"
)]
#[ignore = "K6 T5"]
fn a_setting_not_used_as_written_is_e086_at_its_key() {
    // Each E086 row of the plan: unknown, wrong type, bound, vocabulary, family key, `null` on a defaulted setting, an
    // extension value that is not an object; at specforge.json:L:C with did-you-mean.
    unimplemented!("K6 T5: E086");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a secret written in specforge.json is E087 and no diagnostic echoes its value"
)]
#[ignore = "K6 T5"]
fn a_secret_in_specforge_json_is_e087_and_never_echoed() {
    // `api_key` and `openai_token` are E087; no diagnostic text holds the value; `max_prompt_tokens` is not E087.
    unimplemented!("K6 T5: E087");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "settings for an extension that is not enabled are W166 once and none for one that failed to load"
)]
#[ignore = "K6 T5"]
fn settings_for_an_extension_not_enabled_are_w166() {
    // W166 once per extension no `extensions` entry enables; none for an enabled extension that failed to load.
    unimplemented!("K6 T5: W166");
}
