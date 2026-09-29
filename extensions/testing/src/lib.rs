//! @specforge/testing — the runner-agnostic test vocabulary (ADR 0002).
//!
//! Which kinds accept `verify` obligations, which obligation kinds each
//! allows, and the rules over them (W004 untested testable entity, W009
//! verify kind outside the allowlist) live here rather than in the extensions
//! that own the kinds. Every testable kind is contributed as an enhancement
//! naming its owner, so a kind whose extension a project doesn't use is
//! skipped silently. Test runners (`@specforge/cargo-test`,
//! `@specforge/vitest`, …) build on this vocabulary.
//!
//! The `coverage` pass scores it: intent (verify obligations on testable
//! entities), enforcement (invariants something references), and proof
//! (recorded test results, or formal claims the prove pass entailed).

use specforge_extension_sdk::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

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

        c.pass("coverage", |p| {
            p.after("resolve");
        });
    }
}

/// The kind whose entities carry risk-graded guarantees.
const INVARIANT_KIND: &str = "invariant";
/// The verify kind a proved formal claim discharges.
const PROPERTY_VERIFY_KIND: &str = "property";
const PASSING_STATUS: &str = "pass";

fn at(diagnostic: PassDiagnostic, entity: &PassEntity) -> PassDiagnostic {
    match &entity.span {
        Some(span) => diagnostic.with_span(span.clone()),
        None => diagnostic,
    }
}

/// `coverage` — proof obligations, enforcement, and discharge per entity.
///
/// - A001: testable entity with no verify obligations (no intent)
/// - A002: invariant with no verify obligations (an error when high-risk)
/// - A011: invariant that nothing references (orphan guarantee)
/// - A014: an entity's recorded tests include failures
#[specforge_extension_sdk::compiler_pass(name = "coverage", after = "resolve")]
fn pass_coverage(input: &PassInput) -> PassOutput {
    let proved: Option<BTreeSet<&str>> = input
        .proved_claims
        .as_ref()
        .map(|ids| ids.iter().map(String::as_str).collect());

    let mut findings = Vec::new();
    let mut obligation_kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut testable_total, mut testable_verified) = (0usize, 0usize);
    let (mut with_obligations, mut proven, mut report_failures, mut discharged) =
        (0usize, 0usize, 0usize, 0usize);
    let mut invariant_orphans = 0usize;
    // risk -> (invariants, invariants without obligations)
    let mut invariants: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    let mut entities: Vec<&PassEntity> = input.entities.iter().collect();
    entities.sort_by(|a, b| a.id.cmp(&b.id));
    for entity in entities {
        let (kind, id) = (entity.kind.as_str(), entity.id.as_str());
        let obligations = entity.verify_kinds.len();
        for verify_kind in &entity.verify_kinds {
            *obligation_kinds.entry(verify_kind).or_default() += 1;
        }
        if obligations > 0 {
            with_obligations += 1;
        }

        let recorded = input
            .test_results
            .as_ref()
            .and_then(|r| r.results.get(id))
            .filter(|r| !r.tests.is_empty());
        if let Some(recorded) = recorded {
            let failed: Vec<&str> = recorded
                .tests
                .iter()
                .filter(|t| t.status != PASSING_STATUS)
                .map(|t| t.name.as_deref().unwrap_or("<unnamed>"))
                .collect();
            if failed.is_empty() {
                proven += 1;
            } else {
                report_failures += failed.len();
                findings.push(at(
                    PassDiagnostic::new(
                        "A014",
                        PassSeverity::Error,
                        format!(
                            "{kind} '{id}' has {} failing test(s) in the test results: {}",
                            failed.len(),
                            failed.join(", ")
                        ),
                    ),
                    entity,
                ));
            }
        }

        if proved.as_ref().is_some_and(|ids| ids.contains(id))
            && entity
                .verify_kinds
                .iter()
                .any(|k| k == PROPERTY_VERIFY_KIND)
        {
            discharged += 1;
        }

        if entity.testable {
            testable_total += 1;
            if obligations == 0 {
                findings.push(at(
                    PassDiagnostic::warning(
                        "A001",
                        format!("{kind} '{id}' declares no verify obligations"),
                    )
                    .with_suggestion("add a `verify unit` or `verify property` statement"),
                    entity,
                ));
            } else {
                testable_verified += 1;
            }
        }

        if kind == INVARIANT_KIND {
            let risk = entity
                .fields
                .get("risk")
                .cloned()
                .unwrap_or_else(|| "unspecified".to_string());
            let high_risk = risk == "high";
            let tally = invariants.entry(risk).or_insert((0, 0));
            tally.0 += 1;
            if entity.incoming_edge_count == 0 {
                invariant_orphans += 1;
                findings.push(at(
                    PassDiagnostic::warning(
                        "A011",
                        format!("invariant '{id}' is an orphan guarantee: nothing references it"),
                    )
                    .with_suggestion(
                        "reference it from a behavior (invariants list, requires, ensures, or maintains) or drop the invariant",
                    ),
                    entity,
                ));
            }
            if obligations == 0 {
                tally.1 += 1;
                let (severity, suggestion) = if high_risk {
                    (
                        PassSeverity::Error,
                        "high-risk invariant: add at least one `verify property` obligation",
                    )
                } else {
                    (
                        PassSeverity::Warning,
                        "add a `verify property` or `verify unit` obligation",
                    )
                };
                findings.push(at(
                    PassDiagnostic::new(
                        "A002",
                        severity,
                        format!("invariant '{id}' declares no verify obligations"),
                    )
                    .with_suggestion(suggestion),
                    entity,
                ));
            }
        }
    }

    let invariant_total: usize = invariants.values().map(|(total, _)| total).sum();
    let test_results = input.test_results.as_ref().map(|report| {
        let tests = report.results.values().flat_map(|e| e.tests.iter());
        serde_json::json!({
            "runner": report.runner,
            "entities_recorded": report.results.len(),
            "tests_recorded": tests.clone().count(),
            "tests_failed": tests.filter(|t| t.status != PASSING_STATUS).count(),
            "entities_proven": proven,
        })
    });
    let summary = serde_json::json!({
        "testable_total": testable_total,
        "testable_verified": testable_verified,
        "obligations": obligation_kinds.values().sum::<usize>(),
        "obligation_kinds": obligation_kinds,
        "invariant_enforced": invariant_total - invariant_orphans,
        "invariant_orphans": invariant_orphans,
        "discharge_funnel": {
            "entities_with_obligations": with_obligations,
            "entities_proven": proven,
            "report_failures": report_failures,
            "formally_discharged": discharged,
        },
        "test_results": test_results,
        "invariants": invariants
            .iter()
            .map(|(risk, (total, unverified))| serde_json::json!({
                "risk": risk,
                "total": total,
                "unverified": unverified,
            }))
            .collect::<Vec<_>>(),
    });
    PassOutput {
        diagnostics: findings,
        summary,
    }
}

fn dispatch(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    match export {
        "__pass_coverage" => Some(specforge_dispatch_pass_coverage(input)),
        _ => None,
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build, handler = dispatch);
