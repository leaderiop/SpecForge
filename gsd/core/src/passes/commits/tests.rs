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

#[test]
fn a_commit_reads_its_linked_by() {
    let input: PassInput = serde_json::from_value(json!({
        "entities": [
            {"id": "0123456789ab", "kind": "commit", "fields": {"subject": "feat(01-01): x", "type": "feat", "linked_by": "Gsd-Task"}},
            {"id": "0123456789ac", "kind": "commit", "fields": {"subject": "docs(01-01): y", "type": "docs"}},
        ],
    }))
    .expect("a pass input");
    let found = commits(&input);
    assert_eq!(found[0].linked_by, Some(LinkedBy::Trailer));
    assert_eq!(found[1].linked_by, None);
    assert_eq!(found[0].kind, Some(CommitType::Feat));
    assert_eq!(found[1].kind, Some(CommitType::Docs));
}
