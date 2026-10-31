//! The settings of `@gsd/security`: the three `workflow.security_*` keys of the security capability and the core
//! manifest's `security.injection_blocking` (read by `@gsd/core`'s injection scanner), plus the auditor's model and
//! variant, which follow `@gsd/core`'s `model_profile` and `workflow.compact_content`.

use specforge_extension_sdk::prelude::*;

/// The auditor, as the model catalog names it (`gsd/parity/upstream/v1.18.0/model-catalog.json`).
pub const AGENT: &str = "gsd-security-auditor";

/// The setting that overrides the auditor's model.
pub const MODEL_SETTING: &str = "model_overrides.gsd-security-auditor";

/// The setting that picks the auditor's body (full or compact); the name `@gsd/core`'s agents use.
pub const VARIANT_SETTING: &str = "specforge.agent_variant";

/// `(profile, host tier)`: the catalog row is quality opus, balanced sonnet, budget sonnet, routing tier heavy;
/// `inherit` presets nothing.
const TIERS: &[(&str, &str)] = &[
    ("quality", "heavy"),
    ("balanced", "standard"),
    ("budget", "standard"),
    ("adaptive", "heavy"),
];

/// The setting that shows `/gsd-secure-phase`: its own, following `@gsd/core`'s `audit_review` cluster (a slash's
/// `when` must be a setting of its own extension, W021, and a preset may follow a peer's bool).
pub const SURFACE_SETTING: &str = "specforge.surface.audit_review";

/// Core's `workflow.text_mode` and `response_language`, which `/gsd-secure-phase` reads: a prompt sees only its own
/// extension's settings, so each is declared here. The bool follows core's through a preset.
// shortcut: a preset follows only an enum or a bool (W021), so the free-text `response_language` is not followed: set
// it under `@gsd/security` too. Upgrade when a prompt can read a peer's setting.
pub const TEXT_MODE: &str = "workflow.text_mode";
pub const RESPONSE_LANGUAGE: &str = "response_language";

pub fn declare(c: &mut ContributionsBuilder) {
    c.setting(TEXT_MODE, |s| {
        s.bool().default(false).describe(
            "Replace AskUserQuestion menus with numbered lists; follows @gsd/core's workflow.text_mode.",
        );
    });
    c.preset("@gsd/core:workflow.text_mode", |p| {
        p.when("true", |e| {
            e.set(TEXT_MODE, true);
        });
    });
    c.setting(RESPONSE_LANGUAGE, |s| {
        s.string()
            .describe("Language of agent responses for /gsd-secure-phase; set it as @gsd/core's response_language.");
    });
    c.setting(SURFACE_SETTING, |s| {
        s.bool().default(true).describe(
            "Show /gsd-secure-phase; follows @gsd/core's audit_review cluster (/gsd-surface).",
        );
    });
    c.preset("@gsd/core:specforge.surface.audit_review", |p| {
        p.when("false", |e| {
            e.set(SURFACE_SETTING, false);
        });
    });
    c.setting("workflow.security_enforcement", |s| {
        s.bool().default(true).gates(&["W301"]).describe(
            "Verify threat mitigations at phase close and block shipping while threats are open.",
        );
    });
    c.setting("workflow.security_asvs_level", |s| {
        s.integer()
            .minimum(1)
            .maximum(3)
            .default(1)
            .describe("OWASP ASVS verification level (1 to 3) the security audit runs at.");
    });
    c.setting("workflow.security_block_on", |s| {
        s.enum_values(&["critical", "high", "medium", "low", "none"])
            .default("high")
            .describe(
                "The lowest severity of an open threat that blocks the phase; none blocks nothing.",
            );
    });
    c.setting("security.injection_blocking", |s| {
        s.bool().default(false).describe(
            "Block (not just warn) when a file the agent reads carries a prompt-injection payload.",
        );
    });
    c.setting(MODEL_SETTING, |s| {
        s.enum_values(&["heavy", "standard", "light"])
            .describe("The host tier of the security auditor's model; set by model_profile.");
    });
    c.setting(VARIANT_SETTING, |s| {
        s.enum_values(&["full", "compact"])
            .default("full")
            .describe(
                "Which body the security auditor renders: full, or compact (codex and cursor).",
            );
    });
    c.preset("@gsd/core:model_profile", |p| {
        for (profile, tier) in TIERS {
            p.when(profile, |e| {
                e.set(MODEL_SETTING, *tier);
            });
        }
    });
    c.preset("@gsd/core:workflow.compact_content", |p| {
        p.when("true", |e| {
            e.set(VARIANT_SETTING, "compact");
        });
    });
}
