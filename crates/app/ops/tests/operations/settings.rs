//! Settings in the operations: analyze gates and raises, a command sees a secret's presence (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "analyze findings are gated and raised as the project's report is"
)]
#[ignore = "K6 T6"]
fn analyze_findings_are_gated_and_raised_too() {
    // The same rule on an analyze pass's findings, before `policy.promote`.
    unimplemented!("K6 T6: analyze effective");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a command and a tool see whether a secret is set and never its value"
)]
#[ignore = "K6 T7"]
fn a_command_sees_secret_presence_not_its_value() {
    // `secrets.api_key` is true with T_API_KEY set; the value is in no recorded input.
    unimplemented!("K6 T7: secrets");
}
