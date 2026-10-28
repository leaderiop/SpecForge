//! `today` and `now` (ADR 0294): one instant, rendered twice.

use super::*;
use chrono::{DateTime, Utc};

fn instant(text: &str) -> DateTime<Utc> {
    text.parse().expect("an instant")
}

#[test]
fn now_is_rfc_3339_utc_with_seconds_and_z() {
    let now = now_of(instant("2026-10-03T12:34:56.789+02:00"));
    assert_eq!(now, "2026-10-03T10:34:56Z");
    assert_eq!(now.len(), 20);
    assert!(DateTime::parse_from_rfc3339(&now).is_ok());
    // Fixed width: it sorts as text.
    assert!(now_of(instant("2026-10-03T09:00:00Z")) < now_of(instant("2026-10-03T10:00:00Z")));
}

#[test]
fn today_and_now_are_one_instant() {
    // 23:59:59.9 is still the 3rd in both renderings; a second clock read could say the 4th.
    let at = instant("2026-10-03T23:59:59.900Z");
    assert_eq!(today_of(at), "2026-10-03");
    assert_eq!(now_of(at), "2026-10-03T23:59:59Z");
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "the CommandInput carries the format the caller asked for and the host's date"
)]
fn a_command_input_carries_now() {
    let runtime = std::sync::Arc::new(fake());
    let fixture = fixture(&runtime);
    run_on(
        &fixture.view(),
        &routed("ok"),
        &Map::new(),
        CommandFormat::Json,
        instant("2026-10-03T23:59:59.900Z"),
    )
    .unwrap();
    let input = last_input(&runtime);
    assert_eq!(input["today"], "2026-10-03");
    assert_eq!(input["now"], "2026-10-03T23:59:59Z");
}
