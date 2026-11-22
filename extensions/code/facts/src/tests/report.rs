//! The report kinds, safe id parts and repeat suffixes.

use crate::*;

#[test]
fn an_id_part_encodes_every_delimiter_whitespace_and_control() {
    assert_eq!(
        id_part("a/b#c:d~e@f?g h%\u{1b}"),
        "a%2Fb%23c%3Ad%7Ee%40f%3Fg%20h%25%1B"
    );
    assert_eq!(id_part("Az09.-_é"), "Az09.-_é");
    assert_eq!(id_part("tab\tnl\n\u{a0}"), "tab%09nl%0A%C2%A0");
}

#[test]
fn a_repeated_id_gets_a_numbered_suffix() {
    let mut repeats = Repeats::default();
    let ids: Vec<String> = ["x", "x", "y", "x"]
        .into_iter()
        .map(|base| repeats.id(base.to_string()))
        .collect();
    assert_eq!(ids, ["x", "x~2", "y", "x~3"]);
}

#[test]
fn the_report_kinds_have_their_schemes() {
    let named: Vec<(&str, &str)> = [
        COVERAGE,
        API_ITEM,
        STATIC_FINDING,
        DEPENDENCY,
        VULNERABILITY,
    ]
    .iter()
    .map(|k| (k.kind, k.scheme))
    .collect();
    assert_eq!(
        named,
        [
            ("coverage", "cov"),
            ("api_item", "api"),
            ("static_finding", "sarif"),
            ("dependency", "dep"),
            ("vulnerability", "vuln"),
        ]
    );
    assert_eq!(UNKNOWN, "unknown");
}
