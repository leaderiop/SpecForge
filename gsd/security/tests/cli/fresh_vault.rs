//! One edited file re-judges exactly the verdicts that read it, on the vault twin with `@gsd/security`: the verifier's
//! (`gsd/E501`) on phase 01's must-haves, each truth's citing the files it is about, and the security workflow's
//! (`gsd.security/W302`) on the closed threats, each reading the `files` its disposition rests on (ADR 0377). The
//! twin records no verdict, so each test records the phase's first.

use serde_json::{Value, json};

use super::harness::{Project, vault};

const PHASE: &str = "v1.0/01";
const W302: &str = "gsd.security/W302";
/// The closed threats of the vault, each owed a mitigation verdict, and the files each names.
const CLOSED: [(&str, &str); 5] = [
    ("v1.0/01/01-01/T-01-01", "src/vault.js"),
    ("v1.0/01/01-01/T-01-02", "src/vault.js"),
    ("v1.0/01/01-01/T-01-SC", "package.json"),
    ("v1.0/01/01-02/T-01-03", "src/cli.js"),
    ("v1.0/01/01-02/T-01-SC", "package.json"),
];
/// 01-01's truth cites `src/vault.js` and `test/vault.test.js`; 01-02's `src/cli.js` and `src/vault.js`.
const SEALED: &str = "v1.0/01/01-01/TR1";
const COMMAND_LINE: &str = "v1.0/01/01-02/TR1";

/// The vault with phase 01 judged: the verifier's verdict on each of its six must-haves and `holds` on each closed
/// threat.
fn judged_phase() -> Project {
    let p = vault().with_extension("@gsd/security", "GSD_SECURITY_WASM");
    for (threat, file) in CLOSED {
        assert_eq!(p.fields_of(threat)["files"], json!([file]), "{threat}");
    }
    assert_eq!(p.verify_phase(PHASE), 6);
    for (threat, _) in CLOSED {
        let run = p.record(threat, W302, "holds", &[]);
        assert_eq!(run.code, 0, "{threat}: {}", run.all());
    }
    p
}

/// Appends one comment line to a file of the project.
fn touch(p: &Project, file: &str) {
    let text = p.read(file);
    assert!(text.ends_with('\n'), "{file} ends with a line break");
    p.write(file, &format!("{text}// edited\n"));
}

/// `(entity, the inputs that changed)` of each verdict `re-verify --stale-only` lists, each stale, in answer order.
fn stale(p: &Project) -> Vec<(String, Vec<String>)> {
    let answer = p.re_verify(PHASE, true);
    answer["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| {
            assert_eq!(item["state"], "stale", "{answer:#}");
            let inputs = item["changed"]
                .as_array()
                .expect("changed")
                .iter()
                .map(|c| c["input"].as_str().expect("an input").to_string())
                .collect();
            (
                item["entity"].as_str().expect("an entity").to_string(),
                inputs,
            )
        })
        .collect()
}

fn item(entity: &str, inputs: &[&str]) -> (String, Vec<String>) {
    (
        entity.to_string(),
        inputs.iter().map(|i| i.to_string()).collect(),
    )
}

/// The verdict of `rule` on `entity`, as `query --inputs` answers it.
fn verdict(p: &Project, entity: &str, rule: &str) -> Value {
    p.inputs(entity, rule)["verdict"].clone()
}

/// The closed threats whose mitigation verdict is not fresh, with the inputs that changed.
fn stale_threats(p: &Project) -> Vec<(String, Value)> {
    CLOSED
        .iter()
        .filter_map(|(threat, _)| {
            let verdict = verdict(p, threat, W302);
            (verdict["state"] != "fresh").then(|| (threat.to_string(), verdict["changed"].clone()))
        })
        .collect()
}

/// The entities `specforge check` reports `gsd.security/W302` on, with its message, sorted.
fn w302(p: &Project) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = p
        .report_all()
        .into_iter()
        .filter(|(code, _)| code == W302)
        .map(|(_, message)| {
            let id = message.split('\'').nth(1).expect("an entity").to_string();
            (id, message)
        })
        .collect();
    found.sort();
    found
}

#[test]
fn nothing_is_stale_once_the_phase_is_judged() {
    let p = judged_phase();
    assert_eq!(stale(&p), []);
    assert_eq!(p.re_verify(PHASE, false)["holding"], 6);
    assert_eq!(stale_threats(&p), []);
    assert_eq!(w302(&p), []);
}

/// `src/cli.js` is cited by 01-02's truth and named by `T-01-03`'s files: those two verdicts go stale, with 01-02's
/// artifact and key link; recording exactly those again silences W302 and leaves every other verdict as it was.
#[test]
fn an_edit_to_the_command_line_stales_its_truth_and_its_threat() {
    let p = judged_phase();
    let before = p.verdicts();
    // six verifier verdicts and five mitigation verdicts
    assert_eq!((before.blocks.len(), before.ledger.len()), (11, 11));
    touch(&p, "src/cli.js");
    let listed = stale(&p);
    assert_eq!(
        listed,
        [
            item("v1.0/01/01-02/AR1", &["file"]),
            item("v1.0/01/01-02/KL1", &["from"]),
            item(COMMAND_LINE, &["cited"]),
        ]
    );
    assert_eq!(verdict(&p, SEALED, "E501")["state"], "fresh");
    let threat = "v1.0/01/01-02/T-01-03";
    assert_eq!(stale_threats(&p), [(threat.to_string(), json!(["files"]))]);
    let reported = w302(&p);
    assert!(
        reported.len() == 1
            && reported[0].0 == threat
            && reported[0].1.contains("is stale: files changed"),
        "{reported:#?}"
    );
    let mut rejudged = Vec::new();
    for (id, _) in &listed {
        let citing = p.citing(id);
        let extra: Vec<&str> = citing.iter().map(String::as_str).collect();
        let run = p.record(id, "E501", "verified", &extra);
        assert_eq!(run.code, 0, "{id}: {}", run.all());
        rejudged.push((id.clone(), "gsd/E501".to_string()));
    }
    let run = p.record(threat, W302, "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    rejudged.push((threat.to_string(), W302.to_string()));
    assert_eq!(stale(&p), []);
    assert_eq!(stale_threats(&p), []);
    assert_eq!(w302(&p), []);
    before.assert_only(&p.verdicts(), &rejudged);
}

/// `test/vault.test.js` is cited by 01-01's truth alone and named by no threat.
#[test]
fn an_edit_to_the_tests_stales_the_sealed_store_truth_only() {
    let p = judged_phase();
    touch(&p, "test/vault.test.js");
    assert_eq!(
        stale(&p),
        [
            item("v1.0/01/01-01/KL1", &["from"]),
            item(SEALED, &["cited"]),
        ]
    );
    assert_eq!(verdict(&p, COMMAND_LINE, "E501")["state"], "fresh");
    assert_eq!(stale_threats(&p), []);
    assert_eq!(w302(&p), []);
}

/// `src/vault.js` is cited by both truths and named by 01-01's two threats on the seal and the auth tag.
#[test]
fn an_edit_to_the_vault_stales_both_truths_and_its_threats() {
    let p = judged_phase();
    touch(&p, "src/vault.js");
    assert_eq!(
        stale(&p),
        [
            item("v1.0/01/01-01/AR1", &["file"]),
            item("v1.0/01/01-01/KL1", &["to"]),
            item(SEALED, &["cited"]),
            item("v1.0/01/01-02/KL1", &["to"]),
            item(COMMAND_LINE, &["cited"]),
        ]
    );
    let sealed = ["v1.0/01/01-01/T-01-01", "v1.0/01/01-01/T-01-02"];
    assert_eq!(
        stale_threats(&p),
        sealed.map(|t| (t.to_string(), json!(["files"])))
    );
    let reported: Vec<String> = w302(&p).into_iter().map(|(id, _)| id).collect();
    assert_eq!(reported, sealed);
}
