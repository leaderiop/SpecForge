//! A JSON key's span, found without a JSON parser that keeps spans (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a JSON key is located at its own text and the path at an offset names the key or value there"
)]
#[ignore = "K6 T5"]
fn a_json_key_is_located_at_its_text() {
    // Property test against serde_json: every key of a generated object is located at its own text (escapes included, the
    // last of a duplicate key wins); `path_at(text, offset)` names the key or value under the cursor.
    unimplemented!("K6 T5: json_span locate and path_at");
}
