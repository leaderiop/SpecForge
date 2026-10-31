//! `/gsd-secure-phase` as the prompt `secure_phase` (plan 16 T22), through the real `specforge` binary over projects
//! that load both extensions: rendered for each host with the settings filled, its steps, its phase picked when none
//! is named, and a golden per host.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::harness::{Project, Run, taskboard};
use super::model::model;

const HOSTS: [&str; 3] = ["claude-code", "codex", "cursor"];

/// What no rendered text may name: GSD's planning folder, its tools script and the paths of its install.
const FORBIDDEN: [&str; 8] = [
    ".planning",
    "gsd-tools",
    "gsd_run",
    "gsd-sdk",
    "SECURITY_FILE",
    "gsd.cjs",
    "~/.claude/gsd-core",
    "$HOME/.claude/gsd-core",
];

fn assert_clean(text: &str) {
    for forbidden in FORBIDDEN {
        assert!(!text.contains(forbidden), "the text names {forbidden}");
    }
}

/// `specforge prompt gsd.<id> --host <host> --format json -- <args>`.
fn prompt(p: &Project, id: &str, args: &[&str], host: &str) -> Run {
    let name = format!("gsd.{id}");
    let mut all = vec![
        "prompt",
        name.as_str(),
        "--host",
        host,
        "--format",
        "json",
        "--",
    ];
    all.extend_from_slice(args);
    p.run(&all)
}

fn payload(run: &Run) -> Value {
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {}", run.stdout))
}

fn text_of(rendered: &Value) -> String {
    rendered["messages"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

const PHASE: &str = "v1.0/01";

/// The twin (a built phase 01 and a plan to build, no security entities) with `@gsd/security` loaded.
fn twin() -> Project {
    taskboard().with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

/// The twin with phase 01 verified, so it is `complete`.
fn done() -> Project {
    let p = twin();
    assert_eq!(p.verify_phase(PHASE), 0);
    p
}

fn render(p: &Project, args: &[&str], host: &str) -> Value {
    payload(&prompt(p, "secure_phase", args, host))
}

#[test]
fn secure_phase_renders_for_three_hosts() {
    let p = model();
    for host in HOSTS {
        let rendered = render(&p, &[PHASE], host);
        let text = text_of(&rendered);
        assert!(rendered["payload"]["prompt"]
            .as_str()
            .is_some_and(|id| id.starts_with("@gsd/security@") && id.ends_with(":secure_phase")));
        assert_eq!(rendered["payload"]["subject"], PHASE, "{host}");
        assert!(text.contains("<specforge_gsd_preamble>"), "{host}");
        assert!(text.contains("<arguments>01</arguments>"), "{host}");
        assert!(text.contains("<purpose>"), "{host}");
        assert_clean(&text);
        assert!(text.contains("`$PHASE_ID` is `v1.0/01`"), "{host}");
        // the template ends the text, whose columns are the kinds' fields
        assert!(
            text.contains("<file path=\"gsd-core/templates/SECURITY.md\">"),
            "{host}"
        );
        match host {
            "codex" => assert!(text.starts_with("<codex_skill_adapter>")),
            "cursor" => assert!(text.starts_with("<cursor_skill_adapter>")),
            _ => assert!(!text.contains("_skill_adapter>")),
        }
    }
}

#[test]
fn the_settings_are_filled_bare_into_the_assignments() {
    let p = model();
    let text = text_of(&render(&p, &[PHASE], "claude-code"));
    assert!(text.contains("SECURITY_ASVS=\"1\""), "{text}");
    assert!(text.contains("SECURITY_BLOCK_ON=\"high\""), "{text}");
    p.settings_of(
        "@gsd/security",
        json!({"workflow.security_asvs_level": 3, "workflow.security_block_on": "medium"}),
    );
    let text = text_of(&render(&p, &[PHASE], "claude-code"));
    assert!(text.contains("SECURITY_ASVS=\"3\""), "{text}");
    assert!(text.contains("SECURITY_BLOCK_ON=\"medium\""), "{text}");
}

#[test]
fn the_steps_are_the_auditor_and_the_two_commands() {
    let p = model();
    let rendered = render(&p, &[PHASE], "claude-code");
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    let of = |id: &str| {
        steps
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or_else(|| panic!("no step {id}"))
    };
    assert_eq!(of("audit")["agent"], "gsd-security-auditor");
    assert_eq!(of("record")["tool"], "specforge.gsd.record_security_audit");
    assert_eq!(of("accept")["tool"], "specforge.gsd.accept_risk");
    assert!(of("accept")["title"].as_str().is_some_and(
        |t| t.contains("in their terminal") && t.contains("an agent's call is refused")
    ));
}

#[test]
fn without_a_phase_the_latest_complete_phase_is_picked() {
    let p = done();
    let rendered = render(&p, &[], "claude-code");
    assert_eq!(rendered["payload"]["subject"], PHASE);
    let text = text_of(&rendered);
    assert!(text.contains("<arguments>01</arguments>"), "{text}");
    assert!(text.contains("`$PHASE_ID` is `v1.0/01`"), "{text}");
}

#[test]
fn without_a_complete_phase_it_is_upstreams_state_c() {
    let p = super::harness::project("empty").with_extension("@gsd/security", "GSD_SECURITY_WASM");
    let run = prompt(&p, "secure_phase", &[], "claude-code");
    let text = text_of(&payload(&run));
    assert!(text.contains("not executed"), "{text}");
    assert!(text.contains("/gsd:execute-phase"), "{text}");
}

fn golden(host: &str, actual: &Value) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/prompts")
        .join(host);
    let path = dir.join("secure_phase.json");
    let text = serde_json::to_string_pretty(actual).expect("JSON prints") + "\n";
    // the prompt id names the extension's version: a bump moves no golden
    let text = text.replace(
        concat!("@gsd/security@", env!("CARGO_PKG_VERSION")),
        "@gsd/security@VERSION",
    );
    if std::env::var_os("GSD_BLESS").is_some() {
        std::fs::create_dir_all(&dir).expect("a golden directory");
        std::fs::write(&path, &text).expect("write the golden");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} is missing: GSD_BLESS=1 writes it", path.display()));
    assert!(
        expected == text,
        "{host}/secure_phase differs: review the change, then GSD_BLESS=1"
    );
}

#[test]
fn secure_phase_goldens() {
    let p = model();
    for host in HOSTS {
        let rendered = render(&p, &[PHASE], host);
        let text = text_of(&rendered);
        let lines: Vec<&str> = text.lines().collect();
        let after = lines
            .iter()
            .position(|l| l.trim() == "</specforge_gsd_preamble>")
            .map_or(0, |at| at + 1);
        golden(
            host,
            &json!({
                "payload": rendered["payload"],
                "head": lines.iter().take(80).collect::<Vec<_>>(),
                "body": lines.iter().skip(after).take(60).collect::<Vec<_>>(),
                "end": &lines[lines.len().saturating_sub(40)..],
                "lines": lines.len(),
                "bytes": text.len(),
                "sha256": format!("{:x}", Sha256::digest(text.as_bytes())),
            }),
        );
    }
}

/// REVIEW-security-part1 m4: the preamble names this extension, not core: its settings command is accepted, its
/// harness-field writer is its own, and the table has the security file.
#[test]
fn the_preamble_is_security_s_not_cores() {
    let p = model();
    for host in HOSTS {
        let text = text_of(&render(&p, &[PHASE], host));
        let head = text
            .split("</specforge_gsd_preamble>")
            .next()
            .expect("a preamble");
        assert!(
            head.contains("`specforge settings set @gsd/security"),
            "{host}"
        );
        assert!(!head.contains("@gsd/core"), "{host}: {head}");
        assert!(head.contains("`NN-SECURITY.md`"), "{host}");
        assert!(
            head.contains("security_audit, accepted_risk, audit_run"),
            "{host}"
        );
        assert!(head.contains("`trust_boundary`, `threat`"), "{host}");
    }
}
