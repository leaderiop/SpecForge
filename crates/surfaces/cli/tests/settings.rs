//! `specforge settings` (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "settings lists each setting's type, value, origin, default, gates, raises and each secret's status"
)]
#[ignore = "K6 T8"]
fn settings_lists_value_origin_gates_raises_and_secrets() {
    // Human and JSON listing; `--all`; `--extension`.
    unimplemented!("K6 T8: specforge settings");
}
