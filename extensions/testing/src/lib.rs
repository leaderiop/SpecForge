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

/// The reserved statement whose meaning this extension supplies.
const VERIFY_FIELD: &str = "verify";

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
    Testable {
        kind: "failure_mode",
        owner: GOVERNANCE,
        verify_kinds: &["unit", "integration", "property"],
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
                        .field(VERIFY_FIELD)
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
/// The summary's name for obligations written without a kind.
const UNTYPED_OBLIGATION: &str = "untyped";

/// How an entity's recorded tests cover its `verify` obligations.
struct ObligationProof<'a> {
    /// Obligations a passing test names.
    proven: usize,
    /// Obligations no passing test names, in declaration order.
    unproven: Vec<&'a str>,
    /// Obligation texts tests name that the entity doesn't declare.
    undeclared: Vec<&'a str>,
}

/// A test proves an obligation by naming its text; a test that names none
/// counts toward the entity but proves no particular obligation. When the
/// entity's formal claims were entailed, its `verify property` obligations
/// are discharged without a test.
fn obligation_proof<'a>(
    entity: &'a PassEntity,
    tests: &'a [PassTestResult],
    formally_proved: bool,
) -> ObligationProof<'a> {
    let passing: BTreeSet<&str> = tests
        .iter()
        .filter(|t| t.status == PASSING_STATUS)
        .filter_map(|t| t.verify.as_deref())
        .collect();
    let declared: BTreeSet<&str> = entity.verify_texts.iter().map(String::as_str).collect();
    let mut proven = 0;
    let mut unproven = Vec::new();
    for (i, text) in entity.verify_texts.iter().enumerate() {
        let property = entity.verify_kinds.get(i).map(String::as_str) == Some(PROPERTY_VERIFY_KIND);
        if passing.contains(text.as_str()) || (formally_proved && property) {
            proven += 1;
        } else {
            unproven.push(text.as_str());
        }
    }
    let undeclared: BTreeSet<&str> = tests
        .iter()
        .filter_map(|t| t.verify.as_deref())
        .filter(|text| !declared.contains(text))
        .collect();
    ObligationProof {
        proven,
        unproven,
        undeclared: undeclared.into_iter().collect(),
    }
}

fn quoted(texts: &[&str]) -> String {
    texts
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

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
/// - A015: obligations no passing test names (with recorded results)
/// - A016: tests name obligations the entity doesn't declare
///
/// With recorded results, an entity is proven when it has tests, all of
/// them pass, and each of its obligations is named by a passing test.
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
    let mut obligations_proven = 0usize;
    // risk -> (invariants, invariants without obligations)
    let mut invariants: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    let mut entities: Vec<&PassEntity> = input.entities.iter().collect();
    entities.sort_by(|a, b| a.id.cmp(&b.id));
    for entity in entities {
        let (kind, id) = (entity.kind.as_str(), entity.id.as_str());
        let obligations = entity.verify_kinds.len();
        for verify_kind in &entity.verify_kinds {
            // A bare `verify "..."` has no kind.
            let key = if verify_kind.is_empty() {
                UNTYPED_OBLIGATION
            } else {
                verify_kind
            };
            *obligation_kinds.entry(key).or_default() += 1;
        }
        if obligations > 0 {
            with_obligations += 1;
        }

        let tests: &[PassTestResult] = input
            .test_results
            .as_ref()
            .and_then(|r| r.results.get(id))
            .map_or(&[], |r| r.tests.as_slice());
        if input.test_results.is_some() {
            let formally_proved = proved.as_ref().is_some_and(|ids| ids.contains(id));
            let proof = obligation_proof(entity, tests, formally_proved);
            obligations_proven += proof.proven;
            if !proof.unproven.is_empty() {
                findings.push(at(
                    PassDiagnostic::warning(
                        "A015",
                        format!(
                            "{kind} '{id}' has {} obligation(s) no passing test proves: {}",
                            proof.unproven.len(),
                            quoted(&proof.unproven)
                        ),
                    )
                    .with_suggestion(
                        "link a test to each obligation by its text (`verify = \"...\"` in the test's annotation)",
                    ),
                    entity,
                ));
            }
            if !proof.undeclared.is_empty() {
                findings.push(at(
                    PassDiagnostic::warning(
                        "A016",
                        format!(
                            "tests name obligation(s) {kind} '{id}' does not declare: {}",
                            quoted(&proof.undeclared)
                        ),
                    )
                    .with_suggestion(
                        "fix the test's `verify` text to match the spec's statement exactly, or add the statement",
                    ),
                    entity,
                ));
            }
            if !tests.is_empty()
                && proof.unproven.is_empty()
                && tests.iter().all(|t| t.status == PASSING_STATUS)
            {
                proven += 1;
            }
        }
        if !tests.is_empty() {
            let failed: Vec<&str> = tests
                .iter()
                .filter(|t| t.status != PASSING_STATUS)
                .map(|t| t.name.as_deref().unwrap_or("<unnamed>"))
                .collect();
            if !failed.is_empty() {
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
            "obligations_proven": obligations_proven,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage(input: serde_json::Value) -> PassOutput {
        pass_coverage(&serde_json::from_value(input).unwrap())
    }

    fn codes(output: &PassOutput) -> Vec<&str> {
        output.diagnostics.iter().map(|d| d.code.as_str()).collect()
    }

    fn create_user(tests: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "entities": [{
                "id": "create_user", "kind": "behavior", "testable": true,
                "verify_kinds": ["unit", "unit"],
                "verify_texts": ["rejects a duplicate email", "stores a hashed password"]
            }],
            "test_results": {"runner": "cargo-test", "results": {"create_user": {"tests": tests}}}
        })
    }

    #[test]
    fn an_obligation_is_proven_by_a_passing_test_that_names_it() {
        let out = coverage(create_user(serde_json::json!([
            {"name": "dup", "status": "pass", "verify": "rejects a duplicate email"},
            {"name": "hash", "status": "pass", "verify": "stores a hashed password"}
        ])));
        assert!(out.diagnostics.is_empty(), "{:?}", codes(&out));
        assert_eq!(out.summary["discharge_funnel"]["entities_proven"], 1);
        assert_eq!(out.summary["test_results"]["obligations_proven"], 2);
    }

    #[test]
    fn unnamed_and_failing_tests_leave_obligations_unproven() {
        let out = coverage(create_user(serde_json::json!([
            {"name": "any", "status": "pass"},
            {"name": "hash", "status": "fail", "verify": "stores a hashed password"}
        ])));
        assert_eq!(codes(&out), vec!["A015", "A014"]);
        assert!(out.diagnostics[0].message.contains(
            "2 obligation(s) no passing test proves: \"rejects a duplicate email\", \"stores a hashed password\""
        ));
        assert_eq!(out.summary["discharge_funnel"]["entities_proven"], 0);
    }

    #[test]
    fn a_test_naming_an_undeclared_obligation_is_reported() {
        let out = coverage(create_user(serde_json::json!([
            {"name": "dup", "status": "pass", "verify": "rejects a duplicate email"},
            {"name": "hash", "status": "pass", "verify": "stores a hashed pasword"}
        ])));
        assert_eq!(codes(&out), vec!["A015", "A016"]);
        assert!(out.diagnostics[1]
            .message
            .contains("\"stores a hashed pasword\""));
    }

    #[test]
    fn an_entailed_formal_claim_discharges_property_obligations() {
        let out = coverage(serde_json::json!({
            "entities": [{
                "id": "unique_ids", "kind": "invariant", "testable": true, "incoming_edge_count": 1,
                "verify_kinds": ["property", "unit"],
                "verify_texts": ["ids never collide", "a second insert fails"]
            }],
            "test_results": {"results": {}},
            "proved_claims": ["unique_ids"]
        }));
        assert_eq!(codes(&out), vec!["A015"]);
        assert!(out.diagnostics[0].message.contains("1 obligation(s)"));
        assert!(out.diagnostics[0]
            .message
            .contains("\"a second insert fails\""));
    }

    #[test]
    fn without_recorded_results_obligations_are_not_scored() {
        let mut input = create_user(serde_json::json!([]));
        input.as_object_mut().unwrap().remove("test_results");
        let out = coverage(input);
        assert!(out.diagnostics.is_empty(), "{:?}", codes(&out));
        assert!(out.summary["test_results"].is_null());
    }
}
