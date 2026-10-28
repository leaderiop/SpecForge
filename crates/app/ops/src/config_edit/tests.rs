#![allow(clippy::unwrap_used, reason = "a test's unwrap is its assertion")]
use super::*;
use serde_json::json;

const FILE: &str = "{\n  \"name\": \"p\",\n  \"settings\": {\n    \"@t/s\": {\n      \"level\": 1,\n      \"profile\": \"compat\"\n    }\n  },\n  \"extensions\": [\"@t/s\"]\n}\n";

fn set(text: &str, path: &[&str], value: Value) -> String {
    set_member(text, path, &value).unwrap()
}

#[test]
fn a_member_is_replaced_in_place_and_everything_else_stays() {
    let after = set(FILE, &["settings", "@t/s", "level"], json!(2));
    assert_eq!(after, FILE.replace("\"level\": 1", "\"level\": 2"));
}

#[test]
fn a_member_is_inserted_after_its_last_sibling_with_its_indentation() {
    let after = set(FILE, &["settings", "@t/s", "mode"], json!("fast"));
    assert_eq!(
        after,
        FILE.replace(
            "\"profile\": \"compat\"\n",
            "\"profile\": \"compat\",\n      \"mode\": \"fast\"\n"
        )
    );
}

#[test]
fn the_missing_objects_are_created_at_the_end_of_the_nearest_one() {
    let bare = "{\n  \"name\": \"p\"\n}\n";
    let after = set(bare, &["settings", "@t/s", "level"], json!(3));
    assert_eq!(
        after,
        "{\n  \"name\": \"p\",\n  \"settings\": {\n    \"@t/s\": {\n      \"level\": 3\n    }\n  }\n}\n"
    );
    let empty = "{}";
    assert_eq!(
        set(empty, &["settings", "x", "y"], json!(true)),
        "{\n  \"settings\": {\n    \"x\": {\n      \"y\": true\n    }\n  }\n}"
    );
}

#[test]
fn a_list_is_rendered_pretty_at_the_members_depth() {
    let after = set(FILE, &["settings", "@t/s", "tags"], json!(["a", "b"]));
    assert!(
        after.contains("\"tags\": [\n        \"a\",\n        \"b\"\n      ]"),
        "{after}"
    );
}

#[test]
fn the_edit_keeps_the_files_layout() {
    // Tabs, CRLF, a file with no trailing newline, a single-line object.
    let tabs = "{\r\n\t\"settings\": {\r\n\t\t\"x\": {\r\n\t\t\t\"a\": 1\r\n\t\t}\r\n\t}\r\n}";
    let after = set(tabs, &["settings", "x", "b"], json!(2));
    assert_eq!(
        after,
        "{\r\n\t\"settings\": {\r\n\t\t\"x\": {\r\n\t\t\t\"a\": 1,\r\n\t\t\t\"b\": 2\r\n\t\t}\r\n\t}\r\n}"
    );
    let inline = "{\"settings\": {\"x\": {\"a\": 1}}}";
    assert_eq!(
        set(inline, &["settings", "x", "b"], json!(2)),
        "{\"settings\": {\"x\": {\"a\": 1, \"b\": 2}}}"
    );
    // The members the edit did not touch are byte for byte as they were, in their order.
    let after = set(FILE, &["settings", "@t/s", "level"], json!(9));
    assert!(after.starts_with("{\n  \"name\": \"p\",\n  \"settings\""));
    assert!(after.ends_with("\"extensions\": [\"@t/s\"]\n}\n"));
}

#[test]
fn the_same_value_changes_nothing() {
    assert_eq!(set(FILE, &["settings", "@t/s", "level"], json!(1)), FILE);
}

#[test]
fn a_member_is_removed_with_its_comma() {
    let first = remove_member(FILE, &["settings", "@t/s", "level"]).unwrap();
    assert_eq!(first, FILE.replace("\"level\": 1,\n      ", ""));
    let last = remove_member(FILE, &["settings", "@t/s", "profile"]).unwrap();
    assert_eq!(last, FILE.replace(",\n      \"profile\": \"compat\"", ""));
    // The only member leaves an empty object, which stays.
    let only = remove_member(&last, &["settings", "@t/s", "level"]).unwrap();
    assert!(only.contains("\"@t/s\": {}"), "{only}");
    assert_eq!(
        serde_json::from_str::<Value>(&only).unwrap()["settings"]["@t/s"],
        json!({})
    );
}

#[test]
fn removing_what_is_not_written_changes_nothing() {
    assert_eq!(
        remove_member(FILE, &["settings", "@t/s", "gone"]).unwrap(),
        FILE
    );
    assert_eq!(
        remove_member(FILE, &["settings", "@x/y", "a"]).unwrap(),
        FILE
    );
}

#[test]
fn the_edit_that_would_not_round_trip_is_refused_and_writes_nothing() {
    // A text that is not JSON is not edited.
    assert!(set_member("{\"a\": ", &["a"], &json!(1)).is_err());
    assert!(remove_member("{\"a\": ", &["a"]).is_err());
    // A root that is not an object has nowhere to put a member.
    assert!(set_member("[1]", &["settings", "x", "a"], &json!(1)).is_err());
    // The proof itself: a splice that changes more than the one path is refused.
    let expected = json!({"a": 1});
    assert!(prove("{\"a\": 1}", &expected).is_ok());
    assert!(prove("{\"a\": 1, \"b\": 2}", &expected).is_err());
    assert!(prove("{\"a\": ", &expected).is_err());
}

#[test]
fn a_repeated_key_is_set_and_removed_as_serde_json_reads_it() {
    // The last of a repeated key is the live one.
    let repeated = "{\"settings\": {\"x\": {\"a\": 1, \"a\": 2}}}";
    let set = set_member(repeated, &["settings", "x", "a"], &json!(3)).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&set).unwrap()["settings"]["x"]["a"],
        3
    );
    let gone = remove_member(repeated, &["settings", "x", "a"]).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&gone).unwrap()["settings"]["x"],
        json!({})
    );
}
