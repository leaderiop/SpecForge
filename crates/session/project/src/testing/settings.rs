//! The extensions the settings tests share (ADR 0288): `@t/s` declares typed settings, a preset, a secret and the
//! codes a setting gates or a preset raises; `@t/f` follows its `profile`.

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::ExtensionDeclaration;

/// The extension declaring [`settings_extension`].
pub const SETTINGS_EXTENSION: &str = "@t/s";
/// The extension declaring [`follower_extension`].
pub const FOLLOWER_EXTENSION: &str = "@t/f";

/// `@t/s`: `gadget`s, and
///
/// - `flag` a bool gating `W901` and `W903`, `level` an integer from 1 to 3 (default 1), `mode` an enum `a | b`
/// - `models.*` an enum `opus | sonnet` over the keys `planning | execution`, `tiers.*.*` a block `{model R,
///   effort}`, `sections` a block list `{heading R}`, `notes` a string with no default
/// - `profile` an enum `compat | strict` (default `compat`), preset: `strict` sets `flag` true and raises `W902`
///   to an error
/// - the secret `api_key`, read from `T_API_KEY`
/// - a check pass `audit` reporting `W901` and `W902` once, and the rule `W903` on a `gadget` with no
///   obligations
pub fn settings_extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(SETTINGS_EXTENSION, "1.0.0"));
    c.kind("gadget", |k| {
        k.keyword("gadget")
            .testable(true)
            .supports_verify(true)
            .verify_kinds(&["unit"]);
    });
    for code in ["W901", "W902", "W903"] {
        c.code(code, |d| {
            d.title("A settings test finding")
                .explanation("A finding the settings tests gate or raise.");
        });
    }
    c.setting("flag", |s| {
        s.bool()
            .default(false)
            .describe("Report the gated codes")
            .gates(&["W901", "W903"]);
    });
    c.setting("level", |s| {
        s.integer().minimum(1).maximum(3).default(1);
    });
    c.setting("mode", |s| {
        s.enum_values(&["a", "b"]);
    });
    c.setting("models.*", |s| {
        s.enum_values(&["opus", "sonnet"])
            .keys(&[&["planning", "execution"]]);
    });
    c.setting("tiers.*.*", |s| {
        s.block()
            .member("model", |f| {
                f.field_type(FieldType::String).required();
            })
            .member("effort", |f| {
                f.field_type(FieldType::String);
            });
    });
    c.setting("sections", |s| {
        s.block_list().member("heading", |f| {
            f.field_type(FieldType::String).required();
        });
    });
    c.setting("notes", |s| {
        s.string();
    });
    c.setting("profile", |s| {
        s.enum_values(&["compat", "strict"]).default("compat");
    });
    c.preset("profile", |p| {
        p.when("strict", |e| {
            e.set("flag", true).raise("W902", ValidationSeverity::Error);
        });
    });
    c.secret("api_key", "T_API_KEY", "The test API key");
    c.rule("W903", |r| {
        r.check(CheckKind::NoVerifyStatements)
            .target_kind("gadget")
            .message_template("gadget '{id}' declares no obligations");
    });
    c.pass("audit", |p| {
        p.phase("check").run(|_: &PassInput| {
            vec![
                PassDiagnostic::new("W901", PassSeverity::Warning, "the gated finding"),
                PassDiagnostic::new("W902", PassSeverity::Warning, "the raised finding"),
            ]
        });
    });
    c
}

/// `@t/f`, a peer of `@t/s`: its `W904` is raised to an error while `@t/s:profile` is `strict`.
pub fn follower_extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(FOLLOWER_EXTENSION, "1.0.0"));
    c.peer(SETTINGS_EXTENSION, "*");
    c.code("W904", |d| {
        d.title("A follower finding")
            .explanation("A finding a peer's profile raises.");
    });
    c.preset("@t/s:profile", |p| {
        p.when("strict", |e| {
            e.raise("W904", ValidationSeverity::Error);
        });
    });
    c.pass("audit", |p| {
        p.phase("check").run(|_: &PassInput| {
            vec![PassDiagnostic::new(
                "W904",
                PassSeverity::Warning,
                "the follower's finding",
            )]
        });
    });
    c
}

/// Both declarations, `@t/s` first.
pub fn settings_declarations() -> Vec<ExtensionDeclaration> {
    vec![
        settings_extension().declaration(),
        follower_extension().declaration(),
    ]
}

/// The `specforge.json` text enabling both, with `settings` as given.
pub fn settings_config(settings: serde_json::Value) -> serde_json::Value {
    json!({"extensions": [SETTINGS_EXTENSION, FOLLOWER_EXTENSION], "settings": settings})
}
