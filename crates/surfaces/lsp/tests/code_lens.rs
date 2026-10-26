//! The server answers `textDocument/codeLens` from the compile (ADR 0249 D2).

use crate::session::Session;
use serde_json::{Value, json};
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
