use specforge_test::slugify::slugify_verify_description;

#[test]
fn spaces_become_underscores() {
    assert_eq!(
        slugify_verify_description("missing reference produces E001"),
        "missing_reference_produces_e001"
    );
}

#[test]
fn angle_brackets_become_words() {
    assert_eq!(
        slugify_verify_description("p99 < 200ms under 1000 concurrent"),
        "p99_lt_200ms_under_1000_concurrent"
    );
}

#[test]
fn lte_and_gte() {
    assert_eq!(
        slugify_verify_description("latency <= 100ms and throughput >= 500"),
        "latency_lte_100ms_and_throughput_gte_500"
    );
}

#[test]
fn special_chars_stripped() {
    assert_eq!(
        slugify_verify_description("email uniqueness (under concurrent writes!)"),
        "email_uniqueness_under_concurrent_writes"
    );
}

#[test]
fn consecutive_underscores_collapsed() {
    assert_eq!(
        slugify_verify_description("foo   bar"),
        "foo_bar"
    );
}

#[test]
fn empty_string() {
    assert_eq!(slugify_verify_description(""), "");
}

#[test]
fn idempotent() {
    let once = slugify_verify_description("missing reference produces E001");
    let twice = slugify_verify_description(&once);
    assert_eq!(once, twice);
}
