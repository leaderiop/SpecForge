//! A setting name is a dotted name or a family whose `*` segments each stand for one key (ADR 0288).

use specforge_protocol_types::{Family, SettingDescriptor, valid_name};

fn setting(name: &str, keys: &[&[&str]]) -> SettingDescriptor {
    SettingDescriptor {
        name: name.to_string(),
        keys: keys
            .iter()
            .map(|k| k.iter().map(|s| s.to_string()).collect())
            .collect(),
        field_type: "string".to_string(),
        ..Default::default()
    }
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a family matches its keys, closed or open, and an exact name beats it"
)]
fn a_family_matches_its_keys() {
    assert!(Family::matches("models.*", "models.planning"));
    assert!(!Family::matches("models.*", "models"));
    assert!(!Family::matches("models.*", "models.a.b"));
    assert!(Family::matches("tiers.*.*", "tiers.codex.opus"));
    assert!(!Family::matches("tiers.*.*", "tiers.codex"));
    assert!(Family::matches(
        "review.models.*",
        "review.models.kimi-code"
    ));
    assert!(Family::matches("worktree.baseRef", "worktree.baseRef"));
    assert!(!Family::matches("worktree.baseRef", "worktree.baseref"));
    assert_eq!(
        Family::captures("tiers.*.*", "tiers.codex.opus"),
        Some(vec!["codex", "opus"])
    );

    let closed = setting("models.*", &[&["planning", "execution"]]);
    assert!(closed.admits_key("models.planning"));
    assert!(!closed.admits_key("models.review"));
    assert!(setting("models.*", &[]).admits_key("models.review"));
    assert!(setting("tiers.*.*", &[&[], &["opus"]]).admits_key("tiers.any.opus"));
    assert!(!setting("tiers.*.*", &[&[], &["opus"]]).admits_key("tiers.any.haiku"));

    let all = [setting("models.*", &[]), setting("models.default", &[])];
    assert_eq!(
        Family::pick(&all, "models.default").unwrap().name,
        "models.default"
    );
    assert_eq!(Family::pick(&all, "models.other").unwrap().name, "models.*");
    assert!(Family::pick(&all, "nothing").is_none());

    assert!(Family::overlap("a.*", "a.b"));
    assert!(Family::overlap("a.*.c", "a.b.*"));
    assert!(!Family::overlap("a.*", "a.*.c"));

    for good in [
        "profile",
        "workflow.tdd_mode",
        "worktree.baseRef",
        "models.*",
        "a-b.c_d",
    ] {
        assert_eq!(valid_name(good), Ok(()), "{good}");
    }
    for bad in ["", "a..b", ".a", "a*", "a.*b", "a b", "a.b."] {
        assert!(valid_name(bad).is_err(), "{bad}");
    }
}
