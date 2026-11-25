//! The fate table against what the host accepts (ADR 0391 D3, D7): a home that writes names a field its writer lets
//! `@gsd/import` write, and a computed home names a computed field. A plan writing one field the host refuses is
//! refused whole, so the table is checked here, before any reader writes a row.

mod common;

use gsd_import::table::{self, Home, ID, TITLE};

use common::declared;

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
