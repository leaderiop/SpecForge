//! `specforge.json` `settings` is read by the project config (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "settings that is not an object in specforge.json is E069 and the rest of the file is kept"
)]
#[ignore = "K6 T5"]
fn a_settings_value_that_is_not_an_object_is_e069() {
    // `"settings": []` is ConfigProblem::WrongType { key: "settings", expected: "an object" } (E069); name, extensions and the
    // rest of the file are still read.
    unimplemented!("K6 T5: parse_project_config settings");
}
