//! Verdict freshness at the ship gate on the vault twin with `@gsd/security`, under gsd-strict (ADR 0379): once phase
//! 01 is judged, the ship gate reads the verifier's verdicts on its six must-haves, the mitigation verdicts on its five
//! closed threats and the audit's mapping verdict, and core's `gsd/E516` refuses exactly the ones that are not fresh,
//! another extension's as much as core's own. The value is computed from what `specforge` prints, never typed.

use std::collections::BTreeSet;

use serde_json::json;

use super::fresh_vault::{CLOSED, judged_phase};
use super::harness::{Freshness, Project, Verdict};

const PHASE: &str = "v1.0/01";
const E501: &str = "gsd/E501";
const W302: &str = "gsd.security/W302";
const W303: &str = "gsd.security/W303";
/// The vault's phase 01 audit.
const AUDIT: &str = "v1.0/01/01-SECURITY";
/// Phase 01's must-haves, each owed the verifier's verdict.
const MUST_HAVES: [&str; 6] = [
    "v1.0/01/01-01/AR1",
    "v1.0/01/01-01/KL1",
    "v1.0/01/01-01/TR1",
    "v1.0/01/01-02/AR1",
    "v1.0/01/01-02/KL1",
    "v1.0/01/01-02/TR1",
];

fn verdict(rule: &str, entity: &str) -> Verdict {
    (rule.to_string(), entity.to_string())
}

/// The vault with phase 01 judged (the verifier, each closed threat's mitigation and the audit's mapping), then put
/// under gsd-strict: judged first, so no gate stands in the way of the judging.
fn judged_strict() -> Project {
    let p = judged_phase();
    let run = p.record(AUDIT, W303, "mapped", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    p.settings(json!({ "profile": "gsd-strict" }));
    p
}

/// What holds of every measurement: the gate reads the twelve judgements of the judged phase, `gsd/E516` refuses
/// exactly the ones that are not fresh, and its gate passes exactly when every one is fresh.
fn measured(p: &Project) -> Freshness {
    let f = p.ship_freshness(PHASE);
    let mut expected: BTreeSet<Verdict> = MUST_HAVES.iter().map(|e| verdict(E501, e)).collect();
    expected.extend(CLOSED.iter().map(|(threat, _)| verdict(W302, threat)));
    expected.insert(verdict(W303, AUDIT));
    assert_eq!(f.required, expected, "{f:#?}");
    assert_eq!(f.refused_set(), f.not_fresh(), "{f:#?}");
    assert_eq!(f.gate_passes, f.fresh == f.required, "{f:#?}");
    f
}

/// Judges again every verdict `f` finds not fresh, each by its own command: a must-have by the verifier over what it
/// reads, a closed threat by the security auditor (`holds`). A refused record is a gate that cannot be satisfied.
fn judge_again(p: &Project, f: &Freshness) {
    for (rule, entity) in f.not_fresh() {
        let run = match rule.as_str() {
            E501 => {
                let citing = p.citing(&entity);
                let extra: Vec<&str> = citing.iter().map(String::as_str).collect();
                p.record(&entity, "E501", "verified", &extra)
            }
            W302 => p.record(&entity, W302, "holds", &[]),
            _ => panic!("no judge played for {rule} on {entity}"),
        };
        assert_eq!(run.code, 0, "{rule} on {entity}: {}", run.all());
    }
}

/// Appends one comment line to a file of the project.
fn touch(p: &Project, file: &str) {
    p.write(file, &format!("{}// edited\n", p.read(file)));
}

#[test]
fn every_verdict_the_ship_gate_reads_is_fresh_on_the_vault_under_strict() {
    let p = judged_strict();
    let f = measured(&p);
    eprintln!("{}", f.line("vault"));
    assert_eq!(f.ratio(), 1.0, "{f:#?}");
    assert!(f.refused.is_empty() && f.gate_passes, "{f:#?}");
}

/// `package.json` is named by the two supply-chain threats only: their mitigation verdicts go stale and core's ship
/// gate refuses them under gsd-strict, until the security auditor records them again.
#[test]
fn a_stale_mitigation_verdict_refuses_core_ship_under_strict_until_it_is_recorded_again() {
    let p = judged_strict();
    touch(&p, "package.json");
    let stale = measured(&p);
    eprintln!("control, package.json edited: {}", stale.line("vault"));
    let supply_chain: BTreeSet<Verdict> = [
        verdict(W302, "v1.0/01/01-01/T-01-SC"),
        verdict(W302, "v1.0/01/01-02/T-01-SC"),
    ]
    .into();
    assert_eq!(stale.refused_set(), supply_chain, "{stale:#?}");
    assert!(stale.ratio() < 1.0 && !stale.gate_passes, "{stale:#?}");
    judge_again(&p, &stale);
    let again = measured(&p);
    assert_eq!(again.ratio(), 1.0, "{again:#?}");
    assert!(again.refused.is_empty() && again.gate_passes, "{again:#?}");
}

/// `src/cli.js` is read by 01-02's must-haves (the truth cites it, the artifact is it, the key link starts there) and
/// named by the threat on what `list` shows: those four are refused, core's and the extension's alike, until judged
/// again.
#[test]
fn an_edit_to_the_command_line_refuses_ship_for_its_truth_and_its_threat() {
    let p = judged_strict();
    touch(&p, "src/cli.js");
    let stale = measured(&p);
    eprintln!("control, src/cli.js edited: {}", stale.line("vault"));
    let reading: BTreeSet<Verdict> = [
        verdict(E501, "v1.0/01/01-02/AR1"),
        verdict(E501, "v1.0/01/01-02/KL1"),
        verdict(E501, "v1.0/01/01-02/TR1"),
        verdict(W302, "v1.0/01/01-02/T-01-03"),
    ]
    .into();
    assert_eq!(stale.refused_set(), reading, "{stale:#?}");
    assert!(stale.ratio() < 1.0 && !stale.gate_passes, "{stale:#?}");
    judge_again(&p, &stale);
    let again = measured(&p);
    assert_eq!(again.ratio(), 1.0, "{again:#?}");
    assert!(again.refused.is_empty() && again.gate_passes, "{again:#?}");
}
