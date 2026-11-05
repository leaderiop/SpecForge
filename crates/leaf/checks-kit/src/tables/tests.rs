use std::mem::size_of_val;

use super::*;

fn sorted(names: &[&str]) -> bool {
    names.windows(2).all(|pair| pair[0] < pair[1])
}

#[test]
fn every_table_names_its_pinned_source() {
    let pins = [
        (
            iso3166::SOURCE,
            "ISO 3166-1 (ISO 3166 Maintenance Agency)",
            "lukes/ISO-3166-Countries-with-Regional-Codes, master branch",
            "2026-11-04",
        ),
        (
            iso4217::SOURCE,
            "ISO 4217 list one (SIX, the maintenance agency)",
            "2026-09-17",
            "2026-11-04",
        ),
        (
            iana_tz::SOURCE,
            "IANA time zone database",
            "2026f",
            "2026-11-04",
        ),
        (
            e164::SOURCE,
            "ITU-T E.164 assigned country codes",
            "libphonenumber CountryCodeToRegionCodeMap, master branch",
            "2026-11-04",
        ),
    ];
    for (source, standard, edition, retrieved) in pins {
        assert_eq!(
            (source.standard, source.edition, source.retrieved),
            (standard, edition, retrieved)
        );
        assert!(source.url.starts_with("https://"), "{source:?}");
    }
}

#[test]
fn every_table_pins_its_rows() {
    let pins = [
        (
            "iso3166",
            fingerprint(iso3166::ROWS.iter().map(|r| format!("{r:?}"))),
            10495771351113006641_u64,
        ),
        (
            "iso4217",
            fingerprint(iso4217::ROWS.iter().map(|r| format!("{r:?}"))),
            16755032890329361034_u64,
        ),
        (
            "iana_tz",
            fingerprint(
                iana_tz::CANONICAL
                    .iter()
                    .chain(&iana_tz::LINKS)
                    .map(|name| name.to_string()),
            ),
            8290686189694075778_u64,
        ),
        (
            "e164",
            fingerprint(e164::CODES.iter().map(u16::to_string)),
            9542400855707118782_u64,
        ),
    ];
    let changed: Vec<_> = pins
        .iter()
        .filter(|(_, found, pinned)| found != pinned)
        .collect();
    assert!(
        changed.is_empty(),
        "update each one's SOURCE and pin: {changed:?}"
    );
}

#[test]
fn e164_country_codes_fit_in_4_kib() {
    assert!(size_of_val(&e164::CODES) <= 4096);
    assert!(e164::CODES.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn e164_table_reads_a_code_of_each_length() {
    assert_eq!(e164::code_of("14155550100"), Some("1"));
    assert_eq!(e164::code_of("33123456789"), Some("33"));
    assert_eq!(e164::code_of("212612345678"), Some("212"));
    assert_eq!(e164::code_of("999"), None);
    assert_eq!(e164::code_of(""), None);
    // A byte that is no digit is no code, and does not split a character.
    assert_eq!(e164::code_of("é1"), None);
}

#[test]
fn iana_tz_holds_the_backward_links() {
    assert!(iana_tz::is_canonical("Africa/Casablanca"));
    assert!(iana_tz::is_link("US/Pacific"));
    assert!(!iana_tz::is_canonical("US/Pacific"));
    assert!(!iana_tz::is_canonical("Mars/Olympus"));
    assert!(!iana_tz::is_link("Mars/Olympus"));
    assert!(sorted(&iana_tz::CANONICAL) && sorted(&iana_tz::LINKS));
}

#[test]
fn iso4217_lists_active_codes_only() {
    assert!(iso4217::is_alpha("MAD"));
    assert!(iso4217::is_numeric("504"));
    // Withdrawn for the euro in 2002.
    assert!(!iso4217::is_alpha("FRF"));
    assert!(iso4217::ROWS.windows(2).all(|pair| pair[0].0 < pair[1].0));
}

#[test]
fn iso3166_table_reads_each_form() {
    assert!(iso3166::is_alpha2("MA"));
    assert!(iso3166::is_alpha3("MAR"));
    assert!(iso3166::is_numeric("504"));
    assert!(!iso3166::is_alpha2("ZZ"));
    assert!(iso3166::ROWS.windows(2).all(|pair| pair[0].0 < pair[1].0));
}
