//! The registry checks every settings item against K3's field forms and K10's code book (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "the registry refuses a malformed setting, preset or secret with W021 and keeps the rest"
)]
#[ignore = "K6 T4"]
fn the_registry_refuses_a_malformed_setting() {
    // Every W021 row of the plan: empty segment, two items of one name, overlapping families, `keys` count, a reference
    // type, a default the type does not admit, `gates` on a non-bool or an uncatalogued code, a preset source that is not an enum or
    // bool, a severity not above the code letter, a secret env that is not [A-Z_][A-Z0-9_]*. Each drops only its item.
    unimplemented!("K6 T4: SettingsRegistry");
}
