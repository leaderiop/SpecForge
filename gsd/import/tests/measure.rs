//! The fate table against what the host accepts (ADR 0391 D3, D7): a home that writes names a field its writer lets
//! `@gsd/import` write, and a computed home names a computed field. A plan writing one field the host refuses is
//! refused whole, so the table is checked here, before any reader writes a row.

use std::collections::BTreeMap;

use gsd_import::table::{self, Home, ID, TITLE};
use serde_json::Value;
use specforge_extension_sdk::testing::MockHost;

/// `kind.field` (or `kind.block.member`) → its writer (`""` for the author), from one `describe entities` answer.
fn writers(entities: &Value, into: &mut BTreeMap<String, String>) {
    let writer = |f: &Value| f["writer"].as_str().unwrap_or_default().to_string();
    for kind in entities["items"].as_array().expect("items") {
        let name = kind["name"].as_str().expect("a kind name");
        for field in kind["fields"].as_array().expect("fields") {
            let field_name = field["name"].as_str().expect("a field name");
            into.insert(format!("{name}.{field_name}"), writer(field));
            for member in field["members"].as_array().into_iter().flatten() {
                let member_name = member["name"].as_str().expect("a member name");
                into.insert(format!("{name}.{field_name}.{member_name}"), writer(member));
            }
        }
    }
}

/// The writers of every field of `@gsd/core` (its kinds golden) and of `@gsd/security` (its declaration): the import's
/// peers, whose harness fields it may write (ADR 0391 D7).
fn declared() -> BTreeMap<String, String> {
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../core/tests/golden/kinds.json"
        ))
        .expect("the kinds golden"),
    )
    .expect("the golden is JSON");
    let security: Value = serde_json::from_str(
        &MockHost::new(gsd_security::build())
            .describe_json("entities")
            .expect("@gsd/security describes its entities"),
    )
    .expect("describe answers JSON");
    let mut out = BTreeMap::new();
    writers(&golden["entities"], &mut out);
    writers(&security, &mut out);
    out
}

#[test]
fn every_home_is_declared() {
    let declared = declared();
    let (mut writes, mut computed) = (0, 0);
    for row in table::rows() {
        for home in row.homes {
            let (kind, field, want): (&str, &str, &[&str]) = match *home {
                Home::To { field, .. } | Home::Filled { field, .. }
                    if field == ID || field == TITLE =>
                {
                    continue;
                }
                Home::To { kind, field, .. } | Home::Filled { kind, field, .. } => {
                    writes += 1;
                    (kind, field, &["", "harness"])
                }
                Home::Computed { kind, field } => {
                    computed += 1;
                    (kind, field, &["computed"])
                }
                Home::Redundant(_) | Home::Reported(_) | Home::Ignored(_) | Home::Runtime(_) => {
                    continue;
                }
            };
            let at = format!("{} `{}`: {kind}.{field}", row.file, row.key);
            let writer = declared
                .get(&format!("{kind}.{field}"))
                .unwrap_or_else(|| panic!("{at} is no field of @gsd/core or @gsd/security"));
            assert!(
                want.contains(&writer.as_str()),
                "{at} is written by `{writer}`, not {want:?}"
            );
        }
    }
    assert!(
        writes > 100 && computed > 20,
        "{writes} writes, {computed} computed"
    );
}
