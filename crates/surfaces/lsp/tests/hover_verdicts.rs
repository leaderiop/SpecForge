//! The hover on an entity lists its verdicts (ADR 0290).

use specforge_test_macros::test as spec;

use crate::served::Served;

fn judged_served(extra: &str) -> Served {
    use specforge_project::testing::{GSD_JUDGED, GSD_JUDGED_SPEC, declare_gsd_judged};
    Served::new(&[("a.spec", &format!("{GSD_JUDGED_SPEC}{extra}"))])
        .extension(GSD_JUDGED, declare_gsd_judged)
        .open(&["a.spec"])
}

#[ignore = "K7 T12"]
#[spec(
    behavior = "show_verdict_state",
    verify = "the hover lists an entity's verdicts with their state and the inputs that changed"
)]
fn hover_shows_a_verdicts_state_and_changed_inputs() {
    let served = judged_served("");
    let text = served
        .hover_on("a.spec", "03-02")
        .expect("a hover on the plan");
    assert!(text.contains("Verdicts"), "{text}");
    assert!(text.contains("`gsd/W311` · no verdict"), "{text}");
}
