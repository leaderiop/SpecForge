use super::*;

fn facts() -> Facts {
    Facts {
        pending: false,
        blocked: false,
        unexcused_skip: false,
        unresolved_issue: false,
        open_gaps: Vec::new(),
    }
}

#[test]
fn pending_is_testing() {
    assert_eq!(
        status(&Facts {
            pending: true,
            ..facts()
        }),
        "testing"
    );
}

#[test]
fn a_session_with_nothing_blocking_is_complete() {
    assert_eq!(status(&facts()), "complete");
}

#[test]
fn a_deferred_follow_up_skip_passes() {
    assert!(is_deferred(Some("Deferred follow-up: needs a device")));
    assert!(is_deferred(Some("deferred FOLLOW-UP")));
    assert!(is_deferred(Some("\"Deferred follow-up: a device\"")));
    assert!(!is_deferred(Some("  deferred follow-up")));
    assert!(!is_deferred(Some("Deferred follow-ups to come")));
    assert!(!is_deferred(Some("Not today")));
    assert!(!is_deferred(None));
    assert_eq!(
        status(&Facts {
            unexcused_skip: true,
            ..facts()
        }),
        "partial"
    );
}

#[test]
fn a_resolved_issue_passes() {
    assert_eq!(
        status(&Facts {
            unresolved_issue: true,
            ..facts()
        }),
        "partial"
    );
    assert_eq!(
        status(&Facts {
            unresolved_issue: false,
            ..facts()
        }),
        "complete"
    );
}

#[test]
fn open_gaps_with_root_causes_are_diagnosed() {
    let blocked = Facts {
        blocked: true,
        unresolved_issue: true,
        ..facts()
    };
    assert_eq!(
        status(&Facts {
            open_gaps: vec![true, true],
            ..blocked_like(&blocked)
        }),
        "diagnosed"
    );
    assert_eq!(
        status(&Facts {
            open_gaps: vec![true, false],
            ..blocked_like(&blocked)
        }),
        "partial"
    );
}

fn blocked_like(f: &Facts) -> Facts {
    Facts {
        pending: f.pending,
        blocked: f.blocked,
        unexcused_skip: f.unexcused_skip,
        unresolved_issue: f.unresolved_issue,
        open_gaps: Vec::new(),
    }
}
