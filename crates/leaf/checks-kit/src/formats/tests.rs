use super::*;

fn refused<T>(result: Result<T, crate::Invalid>) -> crate::Invalid {
    match result {
        Ok(_) => panic!("accepted"),
        Err(invalid) => invalid,
    }
}

#[test]
fn email_reads_an_addr_spec() {
    assert!(email("a.b+c@example.com").is_ok());
    assert!(email("not an address").is_err());
    // Display text and a literal are not addr-specs of a host name.
    assert!(email("A <a@example.com>").is_err());
    assert!(email("a@[127.0.0.1]").is_err());
    assert!(email("a@localhost").is_err());
}

#[test]
fn semver_reads_a_version_and_a_prerelease_when_allowed() {
    assert!(semver("1.2.3", false).is_ok());
    assert!(semver("1.2.3-rc.1+build", true).is_ok());
    assert!(semver("1.2", true).is_err());
    let invalid = refused(semver("1.2.3-rc.1+build", false));
    assert_eq!(invalid.at, Some(5..10));
}

#[test]
fn semver_range_reads_cargos_grammar() {
    assert!(semver_range("^1.2, <2").is_ok());
    assert!(semver_range("*").is_ok());
    assert!(semver_range("1.x.y.z").is_err());
}

#[test]
fn spdx_reads_an_expression_and_points_at_the_bad_identifier() {
    assert!(spdx("MIT OR Apache-2.0", false).is_ok());
    assert!(spdx("GPL-3.0-only WITH Classpath-exception-2.0", false).is_ok());
    assert_eq!(refused(spdx("MIT OR Nope", false)).at, Some(7..11));
    // A deprecated identifier needs the argument.
    assert!(spdx("GPL-2.0", false).is_err());
    assert!(spdx("GPL-2.0", true).is_ok());
}

#[test]
fn spdx_list_version_is_pinned() {
    assert_eq!(SPDX_LIST_VERSION, "3.29.0");
}

#[test]
fn iso_date_reads_the_rfc_3339_profile() {
    assert!(iso_date("2024-02-29", DateForm::Date).is_ok());
    assert!(iso_date("2024-02-29T12:30:00Z", DateForm::DateTime).is_ok());
    assert!(iso_date("2024-02-29T12:30:00.5+01:00", DateForm::Any).is_ok());
    // The calendar decides.
    assert!(iso_date("2023-02-29", DateForm::Any).is_err());
    assert!(iso_date("2024-02-29T25:00:00Z", DateForm::Any).is_err());
    // What jiff reads and RFC 3339 does not.
    assert!(iso_date("2024-02-29 12:30:00Z", DateForm::Any).is_err());
    assert!(iso_date("2024-02-29T12:30:00", DateForm::Any).is_err());
    assert!(iso_date("+002024-02-29", DateForm::Any).is_err());
    // A form refuses the other.
    assert!(iso_date("2024-02-29T12:30:00Z", DateForm::Date).is_err());
    assert!(iso_date("2024-02-29", DateForm::DateTime).is_err());
    // A byte past ten is no char boundary to split at.
    assert!(iso_date("2024-02-2é", DateForm::Any).is_err());
}

#[test]
fn iso_duration_reads_iso_8601_only() {
    assert!(iso_duration("P1Y2M3DT4H5M6S").is_ok());
    assert!(iso_duration("PT15M").is_ok());
    assert!(iso_duration("P").is_err());
    // What jiff reads and the standard does not.
    assert!(iso_duration("2 hours").is_err());
    assert!(iso_duration("-P1D").is_err());
    assert!(iso_duration("p1d").is_err());
}

#[test]
fn cron_reads_the_crontab_form() {
    assert!(cron("*/5 0-6 1,15 JAN MON", false).is_ok());
    assert!(cron("0 0 * * *", false).is_ok());
    assert!(cron("0 0 0 * * *", true).is_ok());
    assert!(cron("0 0 * *", false).is_err());
    assert!(cron("0 0 0 * * *", false).is_err());
    assert!(cron("0 0 * * *", true).is_err());
    assert!(cron("61 0 * * *", false).is_err());
    // Quartz's forms and the nicknames are no crontab.
    assert!(cron("@daily", false).is_err());
    assert_eq!(refused(cron("0 0 L * *", false)).at, Some(4..5));
    assert!(cron("0 0 * * MON#1", false).is_err());
    assert!(cron("0 0 ? * *", false).is_err());
}

#[test]
fn regex_accepts_what_the_kernels_matches_accepts() {
    assert!(regex(r"\p{L}+").is_ok());
    assert!(regex(r"(?i)\w+\b").is_ok());
    let invalid = refused(regex("("));
    assert_eq!(invalid.reason, "unclosed group");
    // A pattern too large for the engine's limit is refused, as `matches` refuses it.
    assert!(regex(r"\w{1000}{1000}").is_err());
}

#[test]
fn regex_agrees_with_the_regex_crate_on_every_refusal() {
    let nested = format!("{}a{}", "(".repeat(300), ")".repeat(300));
    let patterns = [
        "",
        "a|b",
        r"(?i)\w+\b",
        r"\p{Greek}+",
        r"\p{Nope}",
        "(",
        ")",
        "[z-a]",
        "a{2,1}",
        r"\1",
        "(?=a)",
        "(?<n>a)(?<n>b)",
        r"(?-u:\xFF)",
        r"(?-u)\w{1000}{100}",
        r"\w{1000}{1000}",
        r"\w{100}{100}",
        "a{4294967295}",
        r"[\p{L}--\p{Lu}]",
        r"\bé\B",
        "x*?+",
        &nested,
    ];
    for pattern in patterns {
        let ours = regex(pattern).map_err(|invalid| invalid.reason);
        let theirs = ::regex::Regex::new(pattern).map(|_| ()).map_err(|error| {
            let message = error.to_string();
            let reason = message.lines().last().unwrap_or(&message).to_owned();
            reason.strip_prefix("error: ").unwrap_or(&reason).to_owned()
        });
        assert_eq!(ours, theirs, "{pattern}");
    }
}

#[test]
fn uuid_reads_the_hyphenated_form_and_its_version() {
    let v4 = "67e55044-10b1-426f-9247-bb680e5fe0c8";
    assert!(uuid(v4, None).is_ok());
    assert!(uuid(v4, Some(4)).is_ok());
    assert_eq!(refused(uuid(v4, Some(7))).at, Some(14..15));
    assert!(uuid("67e5504410b1426f9247bb680e5fe0c8", None).is_err());
    assert!(uuid("urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8", None).is_err());
    assert!(uuid("00000000-0000-0000-0000-00000000000g", None).is_err());
}

#[test]
fn mime_reads_a_media_type() {
    assert!(mime("text/plain").is_ok());
    assert!(mime("application/json; charset=utf-8").is_ok());
    assert!(mime("text").is_err());
}

#[test]
fn bcp47_reads_form_not_registry() {
    assert!(bcp47("en").is_ok());
    assert!(bcp47("zh-Hant-TW").is_ok());
    // Well-formed, and in no registry.
    assert!(bcp47("xq-Latn").is_ok());
    assert!(bcp47("en_US").is_err());
    assert!(bcp47("").is_err());
}

#[test]
fn iso3166_reads_each_form() {
    assert!(iso3166("MA", CodeForm::Alpha2).is_ok());
    assert!(iso3166("MAR", CodeForm::Alpha3).is_ok());
    assert!(iso3166("504", CodeForm::Numeric).is_ok());
    assert!(iso3166("MAR", CodeForm::Any).is_ok());
    assert!(iso3166("MAR", CodeForm::Alpha2).is_err());
    assert!(iso3166("ma", CodeForm::Any).is_err());
}

#[test]
fn iso4217_reads_active_codes() {
    assert!(iso4217("EUR", CodeForm::Alpha3).is_ok());
    assert!(iso4217("978", CodeForm::Numeric).is_ok());
    assert!(iso4217("EUR", CodeForm::Any).is_ok());
    assert!(iso4217("FRF", CodeForm::Any).is_err());
    assert!(iso4217("EU", CodeForm::Alpha2).is_err());
}

#[test]
fn iana_tz_reads_links_unless_canonical() {
    assert!(iana_tz("Europe/Paris", true).is_ok());
    assert!(iana_tz("US/Pacific", false).is_ok());
    assert!(iana_tz("US/Pacific", true).is_err());
    assert!(iana_tz("Mars/Olympus", false).is_err());
}

#[test]
fn e164_checks_the_country_code_not_the_national_plan() {
    assert!(e164("+14155550100").is_ok());
    // No national plan has this number; the code is assigned, and that is all it asks.
    assert!(e164("+2120000").is_ok());
    assert!(e164("14155550100").is_err());
    assert_eq!(refused(e164("+1415x")).at, Some(5..6));
    assert!(e164("+1234567890123456").is_err());
    assert!(e164("+999123").is_err());
    assert!(e164("+1").is_err());
    assert_eq!(refused(e164("+1é")).at, Some(2..4));
}

#[test]
fn ip_reads_each_version() {
    assert!(ip("192.0.2.1", None).is_ok());
    assert!(ip("2001:db8::1", Some(6)).is_ok());
    assert!(ip("192.0.2.1", Some(6)).is_err());
    assert!(ip("2001:db8::1", Some(4)).is_err());
    assert!(ip("192.0.2.256", None).is_err());
    assert!(ip("010.0.0.1", None).is_err());
}

#[test]
fn cidr_reads_the_prefix_and_the_host_bits() {
    assert!(cidr("192.0.2.0/24", true).is_ok());
    assert!(cidr("2001:db8::/32", true).is_ok());
    assert!(cidr("0.0.0.0/0", true).is_ok());
    assert!(cidr("::/0", true).is_ok());
    assert!(cidr("192.0.2.1/32", true).is_ok());
    assert!(cidr("2001:db8::1/128", true).is_ok());
    assert_eq!(refused(cidr("192.0.2.1/24", true)).at, Some(0..9));
    assert!(cidr("192.0.2.1/24", false).is_ok());
    assert!(cidr("1.2.3.4/0", true).is_err());
    assert_eq!(refused(cidr("192.0.2.0/33", true)).at, Some(10..12));
    assert!(cidr("192.0.2.0/", true).is_err());
    assert!(cidr("192.0.2.0/+8", true).is_err());
    assert!(cidr("192.0.2.0", true).is_err());
    assert!(cidr("nope/8", true).is_err());
}

#[test]
fn hostname_reads_rfc_1123() {
    assert!(hostname("a-b.example.com", false).is_ok());
    assert!(hostname("xn--bcher-kva.example", false).is_ok());
    assert_eq!(refused(hostname("a.-b.c", false)).at, Some(2..4));
    assert!(hostname("bücher.example", false).is_err());
    assert!(hostname("1.2.3.4", false).is_err());
    assert!(hostname("a..b", false).is_err());
    assert!(hostname("", false).is_err());
    assert!(hostname(&"a.".repeat(200), false).is_err());
    assert!(hostname(&"a".repeat(64), false).is_err());
    // A wildcard only when asked for, and only first.
    assert!(hostname("*.example.com", true).is_ok());
    assert!(hostname("*.example.com", false).is_err());
    assert!(hostname("a.*.com", true).is_err());
    assert_eq!(refused(hostname("*.-a.com", true)).at, Some(2..4));
}

#[test]
fn port_reads_a_number_in_range() {
    assert!(port("443", false).is_ok());
    assert!(port("65535", false).is_ok());
    assert!(port("65536", false).is_err());
    assert!(port("0", false).is_err());
    assert!(port("0", true).is_ok());
    assert!(port("+80", false).is_err());
    assert!(port("", true).is_err());
}

#[test]
fn hex_color_reads_css_lengths() {
    for ok in ["#abc", "#abcd", "#aabbcc", "#AABBCC80"] {
        assert!(hex_color(ok).is_ok(), "{ok}");
    }
    assert!(hex_color("abc").is_err());
    assert!(hex_color("#ab").is_err());
    assert_eq!(refused(hex_color("#abg")).at, Some(3..4));
    assert_eq!(refused(hex_color("#aé")).at, Some(2..4));
}
