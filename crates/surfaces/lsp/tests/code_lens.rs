//! The server answers `textDocument/codeLens` from the compile (ADR 0249 D2), with a routed entity's thread (ADR 0330).

use crate::recorder::lenses_refreshed;
use crate::served::Served;
use crate::session::Session;
use serde_json::{Value, json};
use specforge_lsp::ClientSupport;
use specforge_project::testing::GSD_ROUTED_SPEC;
use specforge_test_macros::test as spec;

const TEXT: &str = r#"behavior login "Log \"in\" with {braces}" {
  contract """
verify this line is prose, not an obligation
"""
  verify "rejects a bad password"
}
behavior logout
  "Log out" {
  verify "clears the session"
  verify "is idempotent"
}
behavior uses_login "Uses login" {
  depends_on [login]
}
"#;

/// The lens titles of each line, in answer order.
fn titles_at(lenses: &[Value], line: u64) -> Vec<&str> {
    lenses
        .iter()
        .filter(|l| l["range"]["start"]["line"] == line)
        .map(|l| l["command"]["title"].as_str().unwrap())
        .collect()
}

#[spec(
    behavior = "entity_code_lens",
    verify = "a lens counts an entity's references and verify statements as the compile reads them"
)]
#[tokio::test]
async fn a_lens_counts_what_the_compile_reads() {
    let (mut client, uri, _dir) =
        Session::with_extensions(&["@specforge/software"], "a.spec", TEXT).await;
    let reply = client
        .request(
            "textDocument/codeLens",
            json!({"textDocument": {"uri": uri}}),
        )
        .await;
    let lenses = reply["result"]
        .as_array()
        .unwrap_or_else(|| panic!("{reply}"));
    assert_eq!(
        titles_at(lenses, 0),
        ["1 reference", "1 verify statement"],
        "{reply}"
    );
    assert_eq!(
        titles_at(lenses, 6),
        ["0 references", "2 verify statements"],
        "{reply}"
    );
    assert_eq!(titles_at(lenses, 11), ["0 references"], "{reply}");
    let login = lenses
        .iter()
        .find(|l| l["command"]["title"] == "1 reference")
        .unwrap();
    assert_eq!(login["command"]["command"], "specforge.findReferences");
    assert_eq!(
        login["command"]["arguments"],
        json!([uri, {"line": 0, "character": 9}])
    );
}

#[spec(
    behavior = "entity_code_lens",
    verify = "a spec block's lens is at its string, the name it is written with"
)]
#[tokio::test]
async fn a_spec_blocks_lens_is_at_its_string() {
    let (mut client, uri, _dir) =
        Session::with_extensions(&[], "a.spec", "spec \"p\" {\n  version \"0.1.0\"\n}\n").await;
    let reply = client
        .request(
            "textDocument/codeLens",
            json!({"textDocument": {"uri": uri}}),
        )
        .await;
    let lenses = reply["result"]
        .as_array()
        .unwrap_or_else(|| panic!("{reply}"));
    assert_eq!(lenses.len(), 1, "{reply}");
    assert_eq!(
        lenses[0]["range"],
        json!({"start": {"line": 0, "character": 5}, "end": {"line": 0, "character": 8}}),
        "{reply}"
    );
}

/// A server with no project whose client declared `lens_refresh`, `lens.spec` open and compiled.
fn lensing(lens_refresh: bool) -> Served {
    let mut served = Served::detached().client(ClientSupport {
        lens_refresh,
        ..Default::default()
    });
    let uri = served.uri("lens.spec");
    served.open_document(&uri, "behavior login \"Login\" {\n  contract \"x\"\n}\n");
    served
}

#[spec(
    behavior = "entity_code_lens",
    verify = "a recompile asks only a client that declared refreshSupport to refresh its code lenses"
)]
fn a_recompile_asks_a_lens_refreshing_client_to_refresh() {
    let grown = "behavior login \"Login\" {\n  contract \"x\"\n}\nbehavior b \"B\" {\n  depends_on [login]\n}\n";
    for (support, asked) in [(true, true), (false, false)] {
        let mut served = lensing(support);
        served.edit("lens.spec", grown);
        assert_eq!(lenses_refreshed(&served.sent()), asked, "support {support}");
    }
}

#[spec(
    behavior = "show_verdict_state",
    verify = "a code lens says whether each verdict is fresh, failed or stale, and which inputs changed"
)]
fn a_stale_verdict_has_a_code_lens_naming_its_inputs() {
    use specforge_project::testing::{GSD_JUDGED, GSD_JUDGED_SPEC, declare_gsd_judged};
    let served = crate::served::Served::new(&[("a.spec", GSD_JUDGED_SPEC)])
        .extension(GSD_JUDGED, declare_gsd_judged)
        .open(&["a.spec"]);
    let lenses =
        specforge_lsp::answers::code_lenses(&served.state(), &served.uri("a.spec")).unwrap();
    let at_name: Vec<&str> = lenses
        .iter()
        .filter_map(|l| l.command.as_ref().map(|c| c.title.as_str()))
        .collect();
    assert!(
        at_name.iter().any(|t| t.contains("verdicts missing")),
        "{at_name:?}"
    );
}

/// The titles of the lenses `served` answers for `a.spec`.
fn lens_titles(served: &Served) -> Vec<String> {
    specforge_lsp::answers::code_lenses(&served.state(), &served.uri("a.spec"))
        .unwrap()
        .into_iter()
        .filter_map(|l| l.command.map(|c| c.title))
        .collect()
}

#[spec(
    behavior = "show_trace_thread",
    verify = "a code lens on a routed entity says its thread's summary"
)]
fn a_routed_entity_has_a_thread_lens() {
    let titles = lens_titles(&crate::hover_thread::routed_served(GSD_ROUTED_SPEC));
    let threads: Vec<&String> = titles
        .iter()
        .filter(|t| t.starts_with("thread: "))
        .collect();
    assert_eq!(
        threads,
        [
            "thread: plan 03-02 → T1 → 5e2a91c0d3b4 (Gsd-Task) → js:app/test/auth.test.js#keeps a session (passed) → TR1 missing"
        ],
        "{titles:?}"
    );
}

#[spec(
    behavior = "show_trace_thread",
    verify = "an entity of a kind no route starts at has no thread lens"
)]
fn an_entity_without_a_route_has_no_thread_lens() {
    let served = crate::hover_thread::routed_served(GSD_ROUTED_SPEC);
    let lenses =
        specforge_lsp::answers::code_lenses(&served.state(), &served.uri("a.spec")).unwrap();
    let threaded: Vec<u32> = lenses
        .iter()
        .filter(|l| {
            l.command
                .as_ref()
                .is_some_and(|c| c.title.starts_with("thread"))
        })
        .map(|l| l.range.start.line)
        .collect();
    // Line 2 names the requirement; the plan, task, commit, truth and test have no route.
    assert_eq!(threaded, [2], "{lenses:?}");
}
