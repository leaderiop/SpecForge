//! The parity roster of `@gsd/security` (plan 16 T25): every security item of the vendored gsd-core 1.18.0 inventory is
//! built here, served by `@gsd/core`, or not applicable with a reason, and the numbers of ACCEPTANCE's security table
//! are computed from the tables below.

#[path = "parity/evidence.rs"]
mod evidence;

use serde_json::Value;

const MANIFEST: &str = include_str!(gsd_kit::snapshot!("INVENTORY-MANIFEST.json"));
const CAPABILITY_IDS: &str = include_str!(gsd_kit::snapshot!("capability-ids.json"));
const CAPABILITY: &str = include_str!(gsd_kit::snapshot!("capabilities/security/capability.json"));
const ACCEPTANCE: &str = include_str!("../../parity/ACCEPTANCE.md");

const AGENTS_RS: &str = include_str!("../src/agents.rs");
const PROMPTS_RS: &str = include_str!("../src/prompts.rs");

enum Row {
    /// Built here, in the file whose text names it.
    Built(&'static str, &'static str),
    /// Served by another extension.
    Core(&'static str),
    NotApplicable(&'static str),
}
use Row::{Built, Core, NotApplicable};

/// `(family, item, row)`: every item of the manifest whose name has "secur" in it.
const ROWS: &[(&str, &str, Row)] = &[
    (
        "agents",
        "gsd-security-auditor",
        Built(AGENTS_RS, "gsd-security-auditor"),
    ),
    (
        "agents",
        "gsd-security-auditor.compact",
        Built(AGENTS_RS, "compact"),
    ),
    (
        "commands",
        "/gsd-secure-phase",
        Built(PROMPTS_RS, "secure-phase"),
    ),
    (
        "workflows",
        "secure-phase.md",
        Built(PROMPTS_RS, "secure-phase.md"),
    ),
    (
        "references",
        "security-asvs-levels.md",
        Core("a reference every phase's planner reads; @gsd/core ships it"),
    ),
    (
        "cli_modules",
        "security.cjs",
        NotApplicable(
            "the prompt-injection and path-traversal hardening library of the node CLI; the kernel's host guards and \
             typed commands replace it",
        ),
    ),
];

/// Suites, files and docs of the upstream tree about security that SpecForge has no counterpart of.
const NOT_APPLICABLE: &[(&str, &str)] = &[
    (
        "tests/security.test.cjs",
        "153 cases of the node CLI's hardening library security.cjs, which has no counterpart here",
    ),
    (
        "tests/security-scan.test.cjs",
        "the repository's own CI scan of upstream sources for unsafe patterns, not a product behaviour",
    ),
    (
        "tests/secret-scan-lint.test.cjs",
        "lints upstream's secret scanner script, which SpecForge does not ship",
    ),
    (
        "tests/security-prompt-injection.test.cjs",
        "tests the prompt-injection scanner script of upstream's CI, not the product",
    ),
    (
        "tests/instruction-surface-disclosure.test.cjs",
        "checks upstream's installer output for disclosure, which SpecForge's renderers do not share",
    ),
    (
        "tests/kimi-payload-field-shadowing.test.cjs",
        "a Kimi host payload test; Kimi is a later host, not built",
    ),
    (
        "tests/shadow-report.test.cjs",
        "the node CLI's shadow report, which has no counterpart here",
    ),
    (
        "tests/security-dead-exports.test.cjs",
        "checks exports of security.cjs, which is not ported",
    ),
    (
        "tests/fixtures/adversarial/security",
        "adversarial inputs for the node security.cjs library and its scanners",
    ),
    (
        "scripts/prompt-injection-scan.sh",
        "upstream's CI script for scanning prompts; the kernel scans reads in-process",
    ),
    (
        ".github/workflows/security-scan.yml",
        "upstream's own CI workflow for its scanners, not a product behaviour",
    ),
    (
        "SECURITY.md",
        "upstream's vulnerability-reporting policy, about upstream's repository",
    ),
    (
        "docs/explanation/security-model.md",
        "describes the node CLI's trust model; SpecForge's is the kernel's",
    ),
    (
        "docs/features/security-*.md",
        "feature pages of upstream's documentation site",
    ),
    (
        "docs/security/baseline.md",
        "upstream's CI baseline for its scanners",
    ),
];

fn manifest_items() -> Vec<(String, String)> {
    let m: Value = serde_json::from_str(MANIFEST).expect("manifest");
    let mut out = Vec::new();
    for (family, list) in m["families"].as_object().expect("families") {
        for item in list.as_array().expect("a list") {
            let name = item.as_str().unwrap_or_default();
            if name.contains("secur") {
                out.push((family.clone(), name.to_string()));
            }
        }
    }
    out
}

#[test]
fn every_security_item_of_the_manifest_has_a_row() {
    let mut have: Vec<(String, String)> = ROWS
        .iter()
        .map(|(f, i, _)| (f.to_string(), i.to_string()))
        .collect();
    let mut want = manifest_items();
    have.sort();
    want.sort();
    assert_eq!(have, want);
    for (family, item, row) in ROWS {
        match row {
            Built(text, needle) => assert!(text.contains(needle), "{family} {item}: no `{needle}`"),
            Core(why) | NotApplicable(why) => assert!(why.len() > 30, "{item}: reason too short"),
        }
    }
}

#[test]
fn the_suites_and_files_are_named_with_a_reason() {
    for (path, why) in NOT_APPLICABLE {
        assert!(why.len() > 30, "{path}: reason too short");
    }
    let mut paths: Vec<&str> = NOT_APPLICABLE.iter().map(|(p, _)| *p).collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), NOT_APPLICABLE.len());
}

#[test]
fn the_capability_has_three_hooks_and_is_listed() {
    let ids: Vec<String> = serde_json::from_str(CAPABILITY_IDS).expect("ids");
    assert!(ids.iter().any(|i| i == "security"));
    let cap: Value = serde_json::from_str(CAPABILITY).expect("capability");
    let n = |k: &str| cap[k].as_array().map_or(0, Vec::len);
    // a step at verify:post, a contribution at plan:pre and a gate: each served by `security_hooks`
    assert_eq!((n("steps"), n("contributions"), n("gates")), (1, 1, 1));
    assert_eq!(cap["config"].as_object().expect("config").len(), 3);
}

/// The cells of the row of ACCEPTANCE's security table named `family`.
fn row(family: &str) -> Vec<usize> {
    let table = ACCEPTANCE
        .split("| Security family |")
        .nth(1)
        .and_then(|t| t.split("\n\n").next())
        .expect("the security table");
    let line = table
        .lines()
        .find(|l| l.starts_with(&format!("| {family} |")))
        .unwrap_or_else(|| panic!("no row {family}"));
    line.split('|')
        .skip(2)
        .take(5)
        .map(|c| {
            c.trim()
                .parse()
                .unwrap_or_else(|_| panic!("{family}: `{c}`"))
        })
        .collect()
}

#[test]
fn the_acceptance_counts_match() {
    let count = |family: &str, f: fn(&Row) -> bool| {
        ROWS.iter()
            .filter(|(fam, _, r)| *fam == family && f(r))
            .count()
    };
    let built = |r: &Row| matches!(r, Built(..));
    let core = |r: &Row| matches!(r, Core(_));
    let na = |r: &Row| matches!(r, NotApplicable(_));
    // [total, built, owned elsewhere, not applicable, named open]
    let of = |fam: &str| {
        let (b, c, n) = (count(fam, built), count(fam, core), count(fam, na));
        vec![b + c + n, b, c, n, 0]
    };
    assert_eq!(row("Commands"), of("commands"));
    assert_eq!(row("Agents"), of("agents"));
    assert_eq!(row("Workflows and references"), {
        let (w, r) = (of("workflows"), of("references"));
        w.iter().zip(&r).map(|(a, b)| a + b).collect::<Vec<_>>()
    });
    assert_eq!(row("CLI modules"), of("cli_modules"));
    assert_eq!(row("Capabilities"), vec![1, 1, 0, 0, 0]);
    assert_eq!(row("Hooks"), vec![3, 3, 0, 0, 0]);
    assert_eq!(row("Config keys"), vec![4, 4, 0, 0, 0]);
    let codes = evidence::RULE_EVIDENCE.len();
    assert_eq!(row("Codes"), vec![codes, codes, 0, 0, 0]);
}
