use specforge_effects::http::{BODY_CEILING, guest_body_cap};
use specforge_effects::process::{DEADLINE_CEILING, OUTPUT_CAP, request};

#[test]
fn guest_body_cap_stays_below_memory() {
    for mb in [0, 1, 16, 64, 256, 1024, u64::MAX / 2] {
        let cap = guest_body_cap(mb);
        assert!(cap <= BODY_CEILING);
        assert!(cap <= mb.saturating_mul(1024 * 1024) / 4, "{mb} MiB: {cap}");
    }
    assert_eq!(guest_body_cap(64), 16 * 1024 * 1024);
    assert_eq!(guest_body_cap(1024), BODY_CEILING);
}

#[test]
fn a_request_is_held_to_the_deadline_ceiling() {
    let long = request(["x"], DEADLINE_CEILING * 2);
    assert_eq!(long.deadline_ms, 600_000);
    assert_eq!(long.output_cap, OUTPUT_CAP);
    assert_eq!(
        request(["x"], std::time::Duration::from_millis(5)).deadline_ms,
        5
    );
}
