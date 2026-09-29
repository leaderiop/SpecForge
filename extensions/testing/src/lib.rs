//! @specforge/testing — the runner-agnostic test vocabulary (ADR 0002).
//!
//! Which kinds accept `verify` obligations, which obligation kinds each
//! allows, and the rules over them (W004 untested testable entity, W009
//! verify kind outside the allowlist) live here rather than in the extensions
//! that own the kinds. Every testable kind is contributed as an enhancement
//! naming its owner, so a kind whose extension a project doesn't use is
//! skipped silently. Test runners (`@specforge/cargo-test`,
//! `@specforge/vitest`, …) build on this vocabulary.

use specforge_extension_sdk::prelude::*;

/// A kind that accepts `verify` obligations.
struct Testable {
    kind: &'static str,
    /// Extension that owns the kind.
    owner: &'static str,
    /// Obligation kinds `verify <kind> "..."` may use (W009).
    verify_kinds: &'static [&'static str],
    /// Warn (W004) when an entity of this kind declares no obligations.
    requires_obligations: bool,
}

const SOFTWARE: &str = "@specforge/software";
const GOVERNANCE: &str = "@specforge/governance";

const TESTABLE: &[Testable] = &[
    Testable {
        kind: "behavior",
        owner: SOFTWARE,
        verify_kinds: &["unit", "contract", "integration", "property", "performance"],
        requires_obligations: true,
    },
    Testable {
        kind: "invariant",
        owner: SOFTWARE,
        verify_kinds: &["unit", "integration", "property", "performance", "mutation"],
        requires_obligations: true,
    },
    Testable {
        kind: "event",
        owner: SOFTWARE,
        verify_kinds: &["integration", "unit", "deadlock_free", "liveness"],
        requires_obligations: true,
    },
    Testable {
        kind: "type",
        owner: SOFTWARE,
        verify_kinds: &["unit", "property"],
        requires_obligations: true,
    },
    Testable {
        kind: "port",
        owner: SOFTWARE,
        verify_kinds: &["integration", "unit"],
        requires_obligations: true,
    },
    Testable {
        kind: "constraint",
        owner: GOVERNANCE,
        verify_kinds: &["unit", "integration", "property", "load", "contract"],
        requires_obligations: false,
    },
];

#[specforge_extension_sdk::extension(name = "@specforge/testing", version = "1.0.0")]
struct Testing;

impl Contributions for Testing {
    fn contribute(c: &mut ContributionsBuilder) {
        for owner in [SOFTWARE, GOVERNANCE] {
            c.meta.peer_dependencies.push(PeerDependency {
                name: owner.to_string(),
                version: "^1.0".to_string(),
                optional: true,
            });
        }

        for t in TESTABLE {
            c.enhance(t.kind, t.owner, |e| {
                e.verify_kinds(t.verify_kinds);
            });
            if t.requires_obligations {
                c.rule("W004", |r| {
                    r.check(CheckKind::NoVerifyStatements)
                        .target_kind(t.kind)
                        .severity(ValidationSeverity::Warning)
                        .message_template(
                            "{kind} '{id}' is testable but declares no verify obligations and no gherkin scenario",
                        );
                });
            }
            c.rule("W009", |r| {
                r.check(CheckKind::VerifyKindAllowlist)
                    .target_kind(t.kind)
                    .severity(ValidationSeverity::Warning)
                    .message_template(
                        "entity '{id}' has verify kind '{value}' not in allowed set {allowed}",
                    )
                    .constraint(|k| {
                        k.kind("one_of").values(t.verify_kinds);
                    });
            });
        }
    }
}

fn dispatch(_export: &str, _input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    None
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build, handler = dispatch);
