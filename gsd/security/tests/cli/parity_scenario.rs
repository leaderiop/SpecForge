//! The vault twin answers as gsd-tools did over the vault fixture (plan 16 T24): hooks, threat IDs, the ship gate, the
//! audit trail and the settings, each compared with the golden `record-security.sh` recorded from gsd-core 1.19.0
//! (`gsd/parity/goldens/vault/`). A difference the plan names is normalised here and listed in ACCEPTANCE.

use std::process::Command;

use serde_json::{Value, json};

use super::harness::{Project, taskboard, vault};
use gsd_security::commands::hooks::FRAGMENT;

fn project() -> Project {
    vault().with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

fn golden(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/goldens/vault")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn json_of(run: &super::harness::Run) -> Value {
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()))
}

fn gsd(p: &Project, args: &[&str]) -> Value {
    let mut all = vec!["gsd"];
    all.extend_from_slice(args);
    all.extend_from_slice(&["--format", "json"]);
    json_of(&p.run(&all))
}

/// The entries of a golden's `activeHooks` that are the security capability's.
fn security_entries(golden: &Value) -> Value {
    Value::Array(
        golden["activeHooks"]
            .as_array()
            .expect("activeHooks")
            .iter()
            .filter(|h| h["capId"] == "security")
            .cloned()
            .collect(),
    )
}

#[test]
fn the_hooks_are_the_security_entries_of_render_hooks() {
    let p = project();
    for (point, name) in [
        ("plan:pre", "render-hooks-plan-pre"),
        ("verify:post", "render-hooks-verify-post"),
        ("ship:pre", "render-hooks-ship-pre"),
    ] {
        let mut theirs = security_entries(&golden(name));
        assert_eq!(theirs.as_array().map(Vec::len), Some(1), "{name}");
        // [fragment-repointed]: the plan's threat model is entities here, not a `<threat_model>` block
        if let Some(inline) = theirs[0]["fragment"]["inline"].as_str() {
            assert!(inline.contains("<threat_model>"), "{inline}");
            theirs[0]["fragment"]["inline"] = FRAGMENT.into();
        }
        let ours = gsd(&p, &["security-hooks", point]);
        assert_eq!(ours["activeHooks"], theirs, "{point}");
        // upstream omits skippedHooks without --after-fingerprint; ours is the empty list core's jq adds up
        assert_eq!(ours["skippedHooks"], json!([]), "{point}");
    }
}

#[test]
fn with_enforcement_off_no_security_hook_is_active() {
    let p = project();
    p.settings_of(
        "@gsd/security",
        json!({"workflow.security_enforcement": false}),
    );
    for (point, name) in [
        ("plan:pre", "render-hooks-plan-pre-off"),
        ("verify:post", "render-hooks-verify-post-off"),
        ("ship:pre", "render-hooks-ship-pre-off"),
    ] {
        assert_eq!(security_entries(&golden(name)), json!([]), "{name}");
        assert_eq!(
            gsd(&p, &["security-hooks", point])["activeHooks"],
            json!([]),
            "{point}"
        );
    }
}

#[test]
fn the_settings_are_the_four_config_gets() {
    let p = project();
    let run = p.run(&[
        "settings",
        "--extension",
        "@gsd/security",
        "--format",
        "json",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let listed: Value = serde_json::from_str(&run.stdout).expect("JSON");
    let items = listed["extensions"][0]["settings"]
        .as_array()
        .expect("settings");
    for (key, name) in [
        (
            "workflow.security_enforcement",
            "config-get-security-enforcement",
        ),
        (
            "workflow.security_asvs_level",
            "config-get-security-asvs-level",
        ),
        ("workflow.security_block_on", "config-get-security-block-on"),
        (
            "security.injection_blocking",
            "config-get-injection-blocking",
        ),
    ] {
        let item = items
            .iter()
            .find(|i| i["name"] == key)
            .unwrap_or_else(|| panic!("{key}: {listed}"));
        let value = &item["value"];
        assert_eq!(value, &golden(name), "{key}");
    }
}

/// The ids upstream names a plan by: `01-01-PLAN.md`.
fn as_upstream(duplicates: &Value) -> Value {
    Value::Array(
        duplicates
            .as_array()
            .expect("duplicates")
            .iter()
            .map(|d| {
                let plans: Vec<String> = d["plans"]
                    .as_array()
                    .expect("plans")
                    .iter()
                    .map(|p| format!("{}-PLAN.md", p.as_str().expect("a plan")))
                    .collect();
                json!({"id": d["id"], "plans": plans})
            })
            .collect(),
    )
}

#[test]
fn the_threat_ids_are_the_init_fields() {
    let p = project();
    for (phase, name) in [
        ("01", "init-execute-phase-1"),
        ("02", "init-execute-phase-2"),
    ] {
        let theirs = golden(name);
        let ours = gsd(&p, &["threat-ids", phase]);
        assert_eq!(
            as_upstream(&ours["threat_id_duplicates"]),
            theirs["threat_id_duplicates"],
            "{phase}"
        );
        assert_eq!(
            ours["threat_id_duplicate_count"], theirs["threat_id_duplicate_count"],
            "{phase}"
        );
    }
    // plan-phase answers the same fields for phase 1
    let plan = golden("init-plan-phase-1");
    assert_eq!(
        plan["threat_id_duplicates"],
        golden("init-execute-phase-1")["threat_id_duplicates"]
    );
}

#[test]
fn the_ship_gate_blocks_as_check_predicate_did() {
    let p = project();
    // phase 1: an audit with one blocking threat open; upstream read `threats_open: 1` from its frontmatter
    let theirs = golden("check-predicate-1");
    let ours = gsd(&p, &["security-gate", "01"]);
    assert_eq!(ours["block"], theirs["block"]);
    assert_eq!(
        ours["details"]["threats_open"].to_string(),
        theirs["details"]["actual"].as_str().expect("actual")
    );
    assert_eq!(ours["details"]["code"], "SECURITY_SHIP_GATE_OPEN_THREATS");
    // phase 2: no audit; upstream found no SECURITY.md
    let theirs = golden("check-predicate-2");
    let ours = gsd(&p, &["security-gate", "02"]);
    assert_eq!(ours["block"], theirs["block"]);
    assert_eq!(theirs["details"]["artifactNotFound"], true);
    assert_eq!(ours["details"]["exists"], false);
    assert_eq!(ours["details"]["code"], "SECURITY_SHIP_GATE_NO_REVIEW");
}

#[test]
fn the_audit_trail_appends_once_per_change_as_append_audit_did() {
    let p = project();
    // the fixture's trail ends at (3 found, 1 closed, 2 open); this verdict closes more. It names no duplicate ID but
    // `T-01-SC`, which both plans keep by design (REVIEW-security-part2 M2: that case was once avoided here)
    let verdict = json!({"threats": [
        {"id": "T-01-01", "status": "closed", "evidence": "src/vault.js:20"},
        {"id": "T-01-03", "status": "closed"},
        {"id": "T-01-SC", "status": "closed"},
    ]});
    let record = || {
        let run = p.run(&[
            "gsd",
            "record-security-audit",
            "01",
            "--verdict",
            &verdict.to_string(),
            "--format",
            "json",
        ]);
        json_of(&run)["output"]["appended"].clone()
    };
    // the counts differ from upstream's rows (the twin also holds the supply-chain rows and the duplicate), the
    // decision to append does not
    assert_eq!(record(), golden("append-audit-1")["appended"]);
    assert_eq!(record(), golden("append-audit-2")["appended"]);
}

/// The `SHIP_PRE_HOOKS_JSON` line of core's rendered `ship` prompt, run in bash, gives the security gate.
#[test]
fn the_ship_prompts_hook_line_gives_the_security_gate() {
    let has = |tool: &str| Command::new(tool).arg("--version").output().is_ok();
    if !has("bash") || !has("jq") {
        eprintln!("skipped: bash or jq is absent");
        return;
    }
    // the vault's phase 1 is not verified, so `ship` refuses it (E511); the hook line reads only settings, so the
    // prompt is rendered and the line run on the verified taskboard twin
    let p = taskboard().with_extension("@gsd/security", "GSD_SECURITY_WASM");
    assert_eq!(p.verify_phase("v1.0/01"), 0);
    let rendered = p.run(&[
        "prompt",
        "gsd.ship",
        "--host",
        "claude-code",
        "--format",
        "json",
        "--",
        "v1.0/01",
    ]);
    let text = json_of(&rendered)["messages"][0]["text"]
        .as_str()
        .expect("text")
        .to_string();
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("SHIP_PRE_HOOKS_JSON="))
        .expect("the ship:pre line");
    let bin = tempfile::tempdir().expect("a bin dir");
    std::os::unix::fs::symlink(
        std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"),
        bin.path().join("specforge"),
    )
    .expect("a link");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .current_dir(p.root())
        .env("PATH", path)
        .env("SPECFORGE_WASMTIME_CACHE", super::harness::compile_cache())
        .args(["-c", &format!("{line}\necho \"$SHIP_PRE_HOOKS_JSON\"")])
        .output()
        .expect("bash runs");
    let got: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(
        got["activeHooks"],
        security_entries(&golden("render-hooks-ship-pre"))
    );
}

/// The fixture's spec twin checks with both extensions loaded: its one error is the duplicate threat ID the fixture
/// exists to carry (`T-01-02` in both plans of phase 1, reported once per claim), and every field and enum it writes is
/// `@gsd/security`'s own (an unknown one would be an `E0xx`). Its harness-written fields (a threat's `status` and
/// `evidence`, the audit run's counts) are vouched for by the fixture's `specforge-harness` ledger, as the taskboard's
/// verdicts are; without a line there, `E083`.
#[test]
fn the_twin_checks_but_for_the_duplicate_threat_id() {
    let p = project();
    let errors: Vec<(String, String)> = p
        .report_all()
        .into_iter()
        .filter(|(code, _)| code.starts_with('E') || code.contains("/E"))
        .collect();
    assert_eq!(
        errors.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["gsd.security/E101", "gsd.security/E101"],
        "{errors:#?}"
    );
    // an unvouched harness field is refused
    p.edit(
        "specforge-harness",
        "v1.0/01/01-01/T-01-01 status ",
        "v1.0/01/01-01/T-01-01 STATUS ",
    );
    assert!(
        p.report_all()
            .iter()
            .any(|(code, id)| code == "E083" && id.contains("T-01-01")),
        "{:#?}",
        p.report_all()
    );
}
