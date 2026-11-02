//! The hover on an entity of a kind a route starts at shows its thread (ADR 0330).

use specforge_project::testing::{GSD_JUDGED, GSD_ROUTED_SPEC, gsd_routed_runtime};
use specforge_test_macros::test as spec;

use crate::served::Served;

/// `spec` served over `@gsd/core` with the route `gsd/requirement-thread` (the code lens tests' too).
pub fn routed_served(spec: &str) -> Served {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.spec"), spec).unwrap();
    Served::served_by(dir, gsd_routed_runtime(), &[GSD_JUDGED]).open(&["a.spec"])
}

#[spec(
    behavior = "show_trace_thread",
    verify = "the hover on a routed entity shows its thread"
)]
fn the_hover_on_a_routed_entity_shows_its_thread() {
    let served = routed_served(GSD_ROUTED_SPEC);
    let text = served
        .hover_on("a.spec", "AUTH-07")
        .expect("a hover on the requirement");
    assert!(
        text.contains(
            "**Thread** · `gsd/requirement-thread`\n\n```text\nAUTH-07 → plan 03-02 → T1"
        ),
        "{text}"
    );
    let plan = served
        .hover_on("a.spec", "03-02")
        .expect("a hover on the plan");
    assert!(!plan.contains("**Thread**"), "{plan}");
}
