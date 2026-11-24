use super::*;

fn v(name: &str) -> Version {
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
