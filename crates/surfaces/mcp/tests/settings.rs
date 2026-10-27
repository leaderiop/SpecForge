//! The MCP `specforge.settings` tool (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "settings lists each setting's type, value, origin, default, gates, raises and each secret's status"
)]
#[ignore = "K6 T8"]
fn specforge_settings_replies_with_its_output_schema() {
    // The MCP reply is McpSettingsResult; the output schema golden.
    unimplemented!("K6 T8: specforge.settings");
}
