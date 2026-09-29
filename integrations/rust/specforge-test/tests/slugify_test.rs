use specforge_test::slugify::slugify_verify_description;

#[derive(serde::Deserialize)]
struct Case {
    text: String,
    slug: String,
}

/// The host's `specforge_common::slug` is held to the same vectors, so the
/// two copies of the algorithm can't drift.
#[test]
fn matches_the_shared_slug_vectors() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../crates/specforge-common/tests/fixtures/slug-cases.json");
    let cases: Vec<Case> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        assert_eq!(
            slugify_verify_description(&case.text),
            case.slug,
            "slug of {:?}",
            case.text
        );
    }
}

#[test]
fn idempotent() {
    let once = slugify_verify_description("missing reference produces E001");
    let twice = slugify_verify_description(&once);
    assert_eq!(once, twice);
}
