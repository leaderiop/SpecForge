use super::*;

fn v(name: &str) -> Version<'_> {
    parse(name).expect("a version")
}

#[test]
fn a_milestone_name_with_one_to_three_numeric_parts_is_a_version() {
    assert_eq!(v("v1.7").parts, [1, 7, 0]);
    assert_eq!(v("1.7").parts, [1, 7, 0]);
    assert_eq!(v("v1.7.2").parts, [1, 7, 2]);
    assert_eq!(v("v1").parts, [1, 0, 0]);
}

#[test]
fn any_other_name_is_not_a_version() {
    for name in [
        "mvp", "v1.7-rc1", "v1.2.3.4", "", "v", "v1..2", "vv1.7", "v1.7.", " v1.7", "v+1",
    ] {
        assert!(parse(name).is_none(), "{name:?}");
    }
}

#[test]
fn a_part_not_written_reads_as_zero() {
    assert_eq!(v("v1.0"), v("v1.0.0"));
    assert!(v("v1.7") < v("v1.7.1"));
    assert!(v("v1.10") > v("v1.9"));
}

#[test]
fn compatibility_is_cargos_rule() {
    assert!(compatible(&v("1.7"), &v("1.8")));
    assert!(!compatible(&v("1.7"), &v("2.0")));
    assert!(!compatible(&v("0.7"), &v("0.8")));
    assert!(compatible(&v("0.7.1"), &v("0.7.2")));
    assert!(!compatible(&v("0.0.3"), &v("0.0.4")));
    assert!(compatible(&v("v1"), &v("v1.1")));
}

#[test]
fn a_package_matches_the_milestones_first_two_or_written_parts() {
    for same in ["1.7.0", "1.7.0+b5", "1.7", "1.7.1"] {
        assert!(matches(&v("v1.7"), same), "{same}");
    }
    for other in ["1.7.0-rc.1", "1.8.0", "x", "", "1.7.0.1"] {
        assert!(!matches(&v("v1.7"), other), "{other}");
    }
    assert!(!matches(&v("v1.7.2"), "1.7.0"));
    assert!(matches(&v("v1.7.2"), "1.7.2+build"));
    assert!(matches(&v("v1"), "1.0.4"));
    assert!(!matches(&v("v1"), "1.1.0"));
}

#[test]
fn a_version_prints_as_written_without_its_v() {
    assert_eq!(v("v1.7").to_string(), "1.7");
    assert_eq!(v("1.7.2").to_string(), "1.7.2");
    // CalVer keeps its zero: the tag GSD creates for `v2026.01` is `v2026.01`.
    assert_eq!(v("v2026.01").to_string(), "2026.01");
    assert_eq!(v("v01.7").to_string(), "01.7");
}

#[test]
fn a_part_past_u64_is_not_a_version() {
    assert!(parse("v18446744073709551615").is_some());
    assert!(parse("v18446744073709551616").is_none());
    assert!(parse("v1.99999999999999999999").is_none());
}

#[test]
fn an_all_zero_older_version_is_compatible_with_itself_alone() {
    assert!(compatible(&v("0.0.0"), &v("0.0.0")));
    assert!(compatible(&v("0"), &v("0.0")));
    assert!(!compatible(&v("0.0.0"), &v("0.0.1")));
    assert!(!compatible(&v("0.0"), &v("0.1")));
}

#[test]
fn a_zero_padded_version_compares_by_its_numbers() {
    assert_eq!(v("v1.07"), v("v1.7"));
    assert!(v("v2026.01") < v("v2026.2"));
    assert!(matches(&v("v2026.01"), "2026.1.0"));
}
