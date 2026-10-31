use serde_json::json;
use specforge_ops::host::MergedEntry;

use super::merge;

fn entries() -> Vec<MergedEntry> {
    vec![
        MergedEntry {
            keys: vec!["features".into()],
            name: Some("hooks".into()),
            value: json!(true),
        },
        MergedEntry {
            keys: vec!["mcp_servers".into()],
            name: Some("specforge".into()),
            value: json!({"command": "specforge", "args": ["mcp", "."]}),
        },
    ]
}

const SCAN: &[(&[&str], &str)] = &[(&["mcp_servers"], "specforge"), (&["features"], "hooks")];

#[test]
fn toml_merge_writes_a_new_file() {
    let out = merge(None, &entries(), SCAN, &[]).unwrap().text.unwrap();
    let doc: toml_edit::DocumentMut = out.parse().unwrap();
    assert_eq!(doc["features"]["hooks"].as_bool(), Some(true));
    assert_eq!(
        doc["mcp_servers"]["specforge"]["command"].as_str(),
        Some("specforge")
    );
    assert!(!out.contains("\n[mcp_servers]\n"), "{out}");
}

#[test]
fn toml_merge_leaves_a_users_file_byte_for_byte_after_a_round_trip() {
    for original in [
        "# mine\nmodel = \"gpt-5\"\n\n[features]\nweb = true # keep\n\n[mcp_servers.other]\ncommand = \"x\"\n",
        "model='a'\n",
    ] {
        let first = merge(Some(original), &entries(), SCAN, &[]).unwrap();
        let with = first.text.unwrap();
        assert_ne!(with, original);
        assert_eq!(
            merge(Some(&with), &entries(), SCAN, &first.made)
                .unwrap()
                .text
                .unwrap(),
            with
        );
        assert_eq!(
            merge(Some(&with), &[], SCAN, &first.made)
                .unwrap()
                .text
                .unwrap(),
            original
        );
    }
}

#[test]
fn toml_merge_removes_a_file_that_only_held_our_entries() {
    let first = merge(None, &entries(), SCAN, &[]).unwrap();
    let with = first.text.unwrap();
    assert_eq!(
        first.made,
        [
            vec![],
            vec!["features".to_string()],
            vec!["mcp_servers".to_string()]
        ]
    );
    assert_eq!(
        merge(Some(&with), &[], SCAN, &first.made).unwrap().text,
        None
    );
}

#[test]
fn toml_merge_does_not_take_what_the_user_set() {
    let out = merge(
        Some("[mcp_servers.specforge]\ncommand = \"mine\"\n[features]\nhooks = false\n"),
        &entries(),
        SCAN,
        &[],
    )
    .unwrap();
    assert_eq!(out.not_rendered.len(), 2);
    assert!(out.text.unwrap().contains("\"mine\""));
}

#[test]
fn toml_merge_refuses_what_is_no_toml() {
    assert!(merge(Some("a = "), &entries(), SCAN, &[]).is_err());
}

#[test]
fn a_users_own_specforge_table_under_another_name_is_not_ours() {
    let mine = "[mcp_servers.mine]\ncommand = \"specforge\"\nargs = [\"mcp\", \"/other\"]\n";
    let first = merge(Some(mine), &entries(), SCAN, &[]).unwrap();
    let with = first.text.unwrap();
    assert!(with.contains("/other"));
    assert_eq!(
        merge(Some(&with), &[], SCAN, &first.made)
            .unwrap()
            .text
            .unwrap(),
        mine
    );
}

#[test]
fn a_value_where_a_table_goes_is_refused() {
    let err = merge(Some("mcp_servers = 1\n"), &entries(), SCAN, &[]).unwrap_err();
    assert_eq!(
        err,
        "'mcp_servers' holds a value where install needs a table"
    );
}

#[test]
fn a_table_install_did_not_create_stays_when_emptied() {
    let existing = "[mcp_servers]\n";
    let first = merge(Some(existing), &entries(), SCAN, &[]).unwrap();
    assert_eq!(first.made, [vec!["features".to_string()]]);
    let with = first.text.unwrap();
    let out = merge(Some(&with), &[], SCAN, &first.made)
        .unwrap()
        .text
        .unwrap();
    assert_eq!(out, existing);
}

#[test]
fn a_file_install_created_goes_when_emptied() {
    let first = merge(None, &entries(), SCAN, &[]).unwrap();
    assert!(first.made.contains(&Vec::new()));
    let with = first.text.unwrap();
    assert_eq!(
        merge(Some(&with), &[], SCAN, &first.made).unwrap().text,
        None
    );
}
