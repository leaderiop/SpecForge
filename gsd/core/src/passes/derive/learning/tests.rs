use super::*;

#[test]
fn a_deferred_lesson_reopens_when_its_phase_completes() {
    assert_eq!(status(None, false), "open");
    assert_eq!(status(Some("deferred"), false), "deferred");
    assert_eq!(status(Some("deferred"), true), "open");
    assert_eq!(status(Some("promoted"), true), "promoted");
    assert_eq!(status(Some("dismissed"), false), "dismissed");
}
