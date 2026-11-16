use super::*;

fn set<'a>(ids: &[&'a str]) -> BTreeSet<&'a str> {
    ids.iter().copied().collect()
}

#[test]
fn an_unscoped_holder_names_a_full_path_rooted_or_not() {
    let targets = set(&["v1.0/01/01-01/T1"]);
    assert_eq!(
        written_target(&targets, "RK1", "v1.0/01/01-01/T1"),
        Some("v1.0/01/01-01/T1")
    );
    assert_eq!(
        written_target(&targets, "RK1", "/v1.0/01/01-01/T1"),
        Some("v1.0/01/01-01/T1")
    );
    assert_eq!(
        written_target(&targets, "RK1", "T1"),
        None,
        "T1 is in no scope of an unscoped holder"
    );
    assert_eq!(written_target(&targets, "RK1", "01-01/T1"), None);
}

#[test]
fn a_scoped_holder_names_by_its_chain_nearest_first() {
    let holder = "v1.0/01/01-VERIFICATION";
    let targets = set(&["v1.0/01/01-01/TR1", "v1.0/02", "v1.0/01/X", "X"]);
    assert_eq!(
        written_target(&targets, holder, "01-01/TR1"),
        Some("v1.0/01/01-01/TR1"),
        "in the phase"
    );
    assert_eq!(
        written_target(&targets, holder, "02"),
        Some("v1.0/02"),
        "in the milestone"
    );
    assert_eq!(
        written_target(&targets, holder, "X"),
        Some("v1.0/01/X"),
        "the nearer one"
    );
    assert_eq!(
        written_target(&targets, holder, "/X"),
        Some("X"),
        "rooted: the project"
    );
    assert_eq!(
        written_target(&targets, holder, "TR1"),
        None,
        "a plan's member is not in the verification's chain"
    );
    assert_eq!(
        written_target(&targets, "v1.0/01/01-01", "TR1"),
        Some("v1.0/01/01-01/TR1"),
        "the holder's own member"
    );
}

#[test]
fn an_empty_or_broken_spelling_names_nothing() {
    let targets = set(&["v1.0/01", "01"]);
    for written in ["", "/", "v1.0//01", "/v1.0/01/"] {
        assert_eq!(
            written_target(&targets, "v1.0/02/P", written),
            None,
            "{written:?}"
        );
    }
}
