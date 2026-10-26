//! @software — the software vocabulary, authored with the extension SDK.
//!
//! Its kinds, edges, enhancements and rules are declared with the SDK
//! builders in [`declaration`]; the handshake is derived by the SDK from the
//! extension metadata and the contributions. It also hosts the
//! `validate__*` custom-rule exports (E010/W010).

mod declaration;

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@specforge/software",
    version = "1.0.0",
    description = "Software design: behaviors, events, types and ports, and the checks that keep them consistent"
)]
struct Software;

impl Contributions for Software {
    fn contribute(c: &mut ContributionsBuilder) {
        // Optional: product provides the kinds behind the feature, module
        // and milestone links; governance provides the invariants a behavior
        // enforces; without them those links are inert (I004).
        // Diagrams (`model`, `outline`) draw the extension in this colour.
        c.theme_color("#4a90d9");
        c.optional_peer("@specforge/product", "^1.0")
            .optional_peer("@specforge/governance", "^1.0");
        c.meta.sandbox_policy = Some(SandboxPolicy {
            max_memory_mb: Some(256),
            max_execution_ms: Some(5000),
        });
        // `specforge init` writes this as the starter spec of a project that
        // enables software.
        c.starter_template(include_str!("starter.spec"));

        declaration::declare(c);
    }
}

// ── Custom validators (`check: "custom"` rules) ────────────────────────────
// Each `validate__<rule>` export is declared with its rule (`declaration`):
// it receives the protocol's `ValidatorContext` and answers a
// `ValidatorVerdict` (ADR 0013). The host precomputes everything the
// validator needs, so each is a pure function of the context.

use specforge_extension_sdk::{ValidatorContext, ValidatorVerdict};

/// Reference IDs declared by a field value. The wire value of a
/// reference-list field is either the comma-joined ID string (the host's
/// `join(", ")` stringification) or an array of IDs.
fn ref_ids(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => s
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn resolved_kind<'a>(context: &'a ValidatorContext, id: &str) -> Option<&'a str> {
    context
        .referenced
        .iter()
        .find(|r| r.id == id)
        .and_then(|r| r.kind.as_deref())
}

/// E010: milestone behavior references must exist.
fn validate_milestone_behavior_ranges(context: &ValidatorContext) -> ValidatorVerdict {
    for field in &context.entity.fields {
        if field.key != "behaviors" {
            continue;
        }
        for id in ref_ids(&field.value) {
            if resolved_kind(context, &id).is_none() {
                return ValidatorVerdict::Fail {
                    field: Some("behaviors".into()),
                    value: Some(id),
                };
            }
        }
    }
    ValidatorVerdict::Pass
}

/// W010: type fields may only carry known annotations.
fn validate_type_field_annotations(context: &ValidatorContext) -> ValidatorVerdict {
    const KNOWN: &[&str] = &["readonly", "unique", "optional", "literal"];
    for field in &context.entity.fields {
        for ann in &field.annotations {
            if !KNOWN.contains(&ann.as_str()) {
                return ValidatorVerdict::Fail {
                    field: Some(field.key.clone()),
                    value: Some(format!("@{ann}")),
                };
            }
        }
    }
    ValidatorVerdict::Pass
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod declaration_tests;
#[cfg(test)]
mod validator_tests;
