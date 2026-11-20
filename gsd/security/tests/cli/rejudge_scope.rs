//! The re-verification scope over every single-file edit of the vault twin with `@gsd/security` (ADR 0379): once phase
//! 01 is judged, after each edit the verifier's re-verification (`re-verify <phase> --stale-only`) and the security
//! auditor's (`/gsd-secure-phase`, the verdicts it leaves to judge) together list exactly the verdicts `specforge
//! query <entity> --inputs <rule>` says are not fresh, and the spec's own fields ([`Project::reads`]) agree. The value
//! is computed from what `specforge` prints, never typed.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use super::fresh_vault::judged_phase;
use super::harness::{Project, Verdict};
use super::prompt::left_to_judge;

const PHASE: &str = "v1.0/01";
const W302: &str = "gsd.security/W302";
const W303: &str = "gsd.security/W303";
/// The vault's phase 01 audit.
const AUDIT: &str = "v1.0/01/01-SECURITY";

/// What re-verification lists: the verifier's verdicts `re-verify --stale-only` names and the security verdicts
/// `/gsd-secure-phase` leaves to judge.
fn listed(p: &Project) -> BTreeSet<Verdict> {
    let answer = p.re_verify(PHASE, true);
    let mut listed: BTreeSet<Verdict> = answer["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| {
            let text = |key: &str| {
                i[key]
                    .as_str()
                    .unwrap_or_else(|| panic!("{key}: {answer:#}"))
                    .to_string()
            };
            (text("rule"), text("entity"))
        })
        .collect();
    for rule in [W302, W303] {
        let (left, _) = left_to_judge(p, PHASE, rule);
        listed.extend(left.into_iter().map(|entity| (rule.to_string(), entity)));
    }
    listed
}

#[test]
fn re_verification_and_the_secure_phase_list_exactly_the_stale_verdicts_after_any_single_file_edit()
{
    let p = judged_phase();
    let run = p.record(AUDIT, W303, "mapped", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    p.settings(json!({ "profile": "gsd-strict" }));
    // The universe: every judgement the phase's ship gate reads, six verifier verdicts, five mitigation verdicts and
    // the audit's mapping verdict.
    let universe = p.ship_freshness(PHASE).required;
    let mut counts = BTreeMap::new();
    for (rule, _) in &universe {
        *counts.entry(rule.as_str()).or_insert(0) += 1;
    }
    assert_eq!(
        counts,
        BTreeMap::from([("gsd/E501", 6), (W302, 5), (W303, 1)]),
        "{universe:#?}"
    );
    let files = p.fixture_files();
    let scope = p.rejudge_scope(&files, &universe, &listed);
    eprintln!("{}", scope.line("vault"));
    assert!(scope.exact(), "{scope:#?}");
    assert_eq!(scope.ratio(), 1.0);
    assert_eq!(scope.edits, files.len());
    assert!(scope.quiet >= 1 && scope.partial >= 1, "{scope:#?}");
    for (rule, (listed, stale)) in &scope.by_rule {
        assert_eq!(listed, stale, "{rule}");
    }
    // The independent oracle: each edit stales exactly the verdicts the spec says read the file.
    let reads: Vec<(Verdict, BTreeSet<String>)> = universe
        .iter()
        .map(|(rule, entity)| ((rule.clone(), entity.clone()), p.reads(rule, entity)))
        .collect();
    for file in &files {
        let expected: BTreeSet<Verdict> = reads
            .iter()
            .filter(|(_, read)| read.contains(file))
            .map(|(verdict, _)| verdict.clone())
            .collect();
        assert_eq!(scope.per_edit[file], expected, "{file}");
    }
    let supply_chain: BTreeSet<Verdict> = ["v1.0/01/01-01/T-01-SC", "v1.0/01/01-02/T-01-SC"]
        .iter()
        .map(|threat| (W302.to_string(), threat.to_string()))
        .collect();
    assert_eq!(scope.per_edit["package.json"], supply_chain);
}
