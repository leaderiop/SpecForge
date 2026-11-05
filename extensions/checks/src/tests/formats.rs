//! The formats family: each checker's text, valid and not, with what its arguments change.

use serde_json::json;

use super::{clean, e103, e103_at};

#[test]
fn formats_email() {
    clean("email", "ada@example.com", json!({}));
    e103("email", "ada at example.com", json!({}));
}

#[test]
fn formats_semver() {
    clean("semver", "1.2.3-rc.1", json!({}));
    e103("semver", "1.2", json!({}));
    let at = e103_at("semver", "1.2.3-rc.1", json!({"prerelease": false}));
    assert_eq!(at, [5, 10]);
}

#[test]
fn formats_semver_range() {
    clean("semver_range", "^1.2", json!({}));
    e103("semver_range", "~>", json!({}));
}

#[test]
fn formats_spdx() {
    clean("spdx", "MIT OR Apache-2.0", json!({}));
    e103_at("spdx", "MIT OR Nope-1.0", json!({}));
    e103("spdx", "GPL-2.0", json!({}));
    clean("spdx", "GPL-2.0", json!({"deprecated": true}));
}

#[test]
fn formats_iso_date() {
    clean("iso_date", "2026-11-04", json!({}));
    clean("iso_date", "2026-11-04T10:00:00Z", json!({}));
    e103("iso_date", "2026-13-04", json!({}));
    e103("iso_date", "2026-11-04", json!({"form": "datetime"}));
    e103("iso_date", "2026-11-04T10:00:00Z", json!({"form": "date"}));
}

#[test]
fn formats_iso_duration() {
    clean("iso_duration", "P1DT2H", json!({}));
    e103("iso_duration", "1 day", json!({}));
}

#[test]
fn formats_cron() {
    clean("cron", "*/5 * * * *", json!({}));
    e103("cron", "* * *", json!({}));
    e103("cron", "0 */5 * * * *", json!({}));
    clean("cron", "0 */5 * * * *", json!({"seconds": true}));
}

#[test]
fn formats_regex() {
    clean("regex", r"\p{L}+", json!({}));
    e103("regex", "(", json!({}));
}

#[test]
fn formats_uuid() {
    let v4 = "f47ac10b-58cc-4372-a567-0e02b2c3d479";
    clean("uuid", v4, json!({}));
    clean("uuid", v4, json!({"version": 4}));
    e103("uuid", v4, json!({"version": 7}));
    e103("uuid", "f47ac10b58cc4372a5670e02b2c3d479", json!({}));
}

#[test]
fn formats_mime() {
    clean("mime", "text/html", json!({}));
    e103("mime", "text", json!({}));
}

#[test]
fn formats_bcp47() {
    clean("bcp47", "fr-MA", json!({}));
    e103("bcp47", "not a tag", json!({}));
}

#[test]
fn formats_iso3166() {
    clean("iso3166", "MA", json!({}));
    clean("iso3166", "MAR", json!({"form": "alpha3"}));
    clean("iso3166", "504", json!({"form": "numeric"}));
    e103("iso3166", "MA", json!({"form": "alpha3"}));
    e103("iso3166", "ZZ", json!({}));
}

#[test]
fn formats_iso4217() {
    clean("iso4217", "MAD", json!({}));
    clean("iso4217", "504", json!({"form": "numeric"}));
    e103("iso4217", "XXQ", json!({}));
}

#[test]
fn formats_iana_tz() {
    clean("iana_tz", "Africa/Casablanca", json!({}));
    clean("iana_tz", "US/Pacific", json!({}));
    e103("iana_tz", "US/Pacific", json!({"canonical": true}));
    e103("iana_tz", "Mars/Olympus", json!({}));
}

#[test]
fn formats_e164() {
    clean("e164", "+212522000000", json!({}));
    e103("e164", "0522000000", json!({}));
}

#[test]
fn formats_ip() {
    clean("ip", "192.0.2.1", json!({}));
    clean("ip", "2001:db8::1", json!({"version": "6"}));
    e103("ip", "192.0.2.1", json!({"version": "6"}));
    e103("ip", "192.0.2", json!({}));
}

#[test]
fn formats_cidr() {
    clean("cidr", "192.0.2.0/24", json!({}));
    e103("cidr", "192.0.2.1/24", json!({}));
    clean("cidr", "192.0.2.1/24", json!({"strict": false}));
    assert_eq!(e103_at("cidr", "nope/24", json!({})), [0, 4]);
}

#[test]
fn formats_hostname() {
    clean("hostname", "docs.example.com", json!({}));
    e103("hostname", "*.example.com", json!({}));
    clean("hostname", "*.example.com", json!({"wildcard": true}));
}

#[test]
fn formats_port() {
    clean("port", "8080", json!({}));
    e103("port", "0", json!({}));
    clean("port", "0", json!({"zero": true}));
    e103("port", "70000", json!({}));
}

#[test]
fn formats_hex_color() {
    clean("hex_color", "#a1b2c3", json!({}));
    e103("hex_color", "a1b2c3", json!({}));
}

#[test]
fn formats_leave_an_empty_value_to_required() {
    clean("email", "", json!({}));
}

#[test]
fn formats_carry_their_switch_and_need_no_scope() {
    let declaration = super::declaration();
    let formats: Vec<_> = declaration
        .checkers
        .iter()
        .filter(|checker| checker.when == ["families.formats"])
        .collect();
    assert_eq!(formats.len(), 20);
    assert!(formats.iter().all(|checker| checker.scopes.is_empty()));
    assert!(formats.iter().all(|checker| checker.codes == ["E103"]));
}
