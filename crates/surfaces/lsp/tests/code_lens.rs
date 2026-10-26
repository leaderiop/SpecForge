//! The server answers `textDocument/codeLens` from the compile (ADR 0249 D2).

use crate::recorder::lenses_refreshed;
use crate::served::Served;
use crate::session::Session;
use serde_json::{Value, json};
use specforge_lsp::ClientSupport;
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
