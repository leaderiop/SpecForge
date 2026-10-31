//! `specforge settings` over `@gsd/security`'s four keys.

use super::both;

#[test]
fn the_settings_list_the_four_keys_with_their_defaults() {
    let p = both();
    let out = p.run(&[
        "settings",
        "--extension",
        "@gsd/security",
        "--format",
        "json",
    ]);
    assert_eq!(out.code, 0, "{}", out.all());
    let text = out.stdout;
    for key in [
        "workflow.security_enforcement",
        "workflow.security_asvs_level",
        "workflow.security_block_on",
        "security.injection_blocking",
    ] {
        assert!(text.contains(key), "{key}: {text}");
    }
}

#[test]
fn an_asvs_level_outside_1_to_3_is_refused() {
    let p = both();
    let out = p.run(&[
        "settings",
        "set",
        "@gsd/security",
        "workflow.security_asvs_level",
        "4",
    ]);
    assert_ne!(out.code, 0, "{}", out.all());
    assert!(out.all().contains("E086"), "{}", out.all());
    let ok = p.run(&[
        "settings",
        "set",
        "@gsd/security",
        "workflow.security_asvs_level",
        "2",
    ]);
    assert_eq!(ok.code, 0, "{}", ok.all());
}
