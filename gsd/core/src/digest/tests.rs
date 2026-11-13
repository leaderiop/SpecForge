use super::*;

#[test]
fn fnv1a64_matches_the_published_vectors() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn a_files_digest_ignores_order_and_repeats_and_follows_a_digest() {
    let a = files([("a.js", "sha256:1"), ("b.js", "sha256:2")]);
    assert_eq!(
        a,
        files([
            ("b.js", "sha256:2"),
            ("a.js", "sha256:1"),
            ("a.js", "sha256:1")
        ])
    );
    assert_ne!(a, files([("a.js", "sha256:1"), ("b.js", "sha256:3")]));
}
