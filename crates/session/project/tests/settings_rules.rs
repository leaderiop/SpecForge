//! A setting gates its codes and a preset raises them, once, in the project's report (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a code gated by a false bool setting is reported by no source"
)]
#[ignore = "K6 T6"]
fn a_gated_code_is_reported_by_no_source_while_its_setting_is_false() {
    // W901 (a pass) and W903 (a declarative rule) are gone with `flag` false and back with true.
    unimplemented!("K6 T6: gated codes");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a preset raises a code upward only and the surface's strict applies after"
)]
#[ignore = "K6 T6"]
fn a_preset_raises_a_code_and_strict_applies_after() {
    // W902 is an error under the strict profile, never lowered; `--strict` promotes warnings after the raise.
    unimplemented!("K6 T6: raised codes");
}
