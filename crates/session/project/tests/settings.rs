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
