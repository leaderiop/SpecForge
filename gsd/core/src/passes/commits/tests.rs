use serde_json::json;

use super::*;

#[test]
fn a_commit_is_read_under_its_twelve_hex_name() {
    let input: PassInput = serde_json::from_value(json!({
        "entities": [{"id": "0123456789ab", "kind": "commit", "fields": {"subject": "feat(01-01): x"}}],
    }))
    .expect("a pass input");
    let found = commits(&input);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "0123456789ab");
    assert_eq!(found[0].subject, "feat(01-01): x");
}
