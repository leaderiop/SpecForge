//! The language server serves `specforge.json`'s settings as they are typed (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "completion in specforge.json offers extension names, unwritten setting names and values"
)]
#[ignore = "K6 T9"]
fn config_completion_offers_settings_keys_and_values() {
    // Extension names at the first level; the setting names not yet written (a snippet with the default) and a
    // family's closed keys at the second; enum values and true/false at a value.
    unimplemented!("K6 T9: config completion");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "hover on a setting key shows its type, default, effective value, origin and gated codes"
)]
#[ignore = "K6 T9"]
fn config_hover_shows_value_and_origin() {
    // explicit | preset (profile = strict) | default; gated codes and their state.
    unimplemented!("K6 T9: config hover");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "specforge.json diagnostics follow the unsaved buffer and a save reloads the environment"
)]
#[ignore = "K6 T9"]
fn config_diagnostics_follow_the_buffer_and_reload_on_save() {
    // E086 at its range on an unsaved edit; cleared on fix; a save reloads (a raise is visible in the .spec diagnostics).
    unimplemented!("K6 T9: config diagnostics");
}
