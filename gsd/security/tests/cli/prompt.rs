//! `/gsd-secure-phase` as the prompt `secure_phase`, through the real `specforge` binary over projects
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
        // the template, which names the kinds, ends the text
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

/// The auditor, then the audit recorded, then the verdicts recorded by the orchestrator (the prompt is the judge of
/// `gsd.security/W302` and `W303`), then a person's accepted risk.
#[test]
fn the_steps_are_the_auditor_and_the_three_commands() {
    let p = model();
    let rendered = render(&p, &[PHASE], "claude-code");
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    let ids: Vec<&str> = steps.iter().filter_map(|s| s["id"].as_str()).collect();
    assert_eq!(ids, ["audit", "record", "judge", "accept"]);
    let of = |id: &str| {
        steps
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or_else(|| panic!("no step {id}"))
    };
    assert_eq!(of("audit")["agent"], "gsd-security-auditor");
    assert_eq!(of("record")["tool"], "specforge.gsd.record_security_audit");
    assert_eq!(of("judge")["tool"], "specforge.gsd.record_security_verdict");
    assert!(
        of("judge")["title"].as_str().is_some_and(
            |t| t.contains("specforge gsd record-security-verdict <entity> <W302|W303>")
        ),
        "{steps:#?}"
    );
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
    assert!(text.contains("/gsd-execute-phase"), "{text}");
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

/// The preamble names this extension, not core: its settings command is accepted, its
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

/// Core's `response_language` and `workflow.text_mode` reach `/gsd-secure-phase` as they
/// reach core's prompts (a prompt reads only its own extension's settings, so `@gsd/security` follows them).
#[test]
fn the_prompt_honours_core_response_language() {
    let p = model();
    // text mode follows core's setting
    p.settings_of("@gsd/core", json!({"workflow.text_mode": true}));
    let text = text_of(&render(&p, &[PHASE], "claude-code"));
    assert!(text.contains("\"workflow.text_mode\": true"), "{text}");
    // a free-text setting cannot be followed (W021): it is set under `@gsd/security`
    p.settings_of("@gsd/security", json!({"response_language": "French"}));
    let text = text_of(&render(&p, &[PHASE], "claude-code"));
    assert!(text.contains("RESPONSE_LANGUAGE=\"French\""), "{text}");
}

/// What `secure_phase`'s Rust side writes names no GSD file (ADR 0332 D8): no rendered line that is no asset line
/// and no row of a preamble table names one.
#[test]
fn the_rendered_tail_names_no_gsd_file() {
    let text = text_of(&render(&model(), &[PHASE], "claude-code"));
    let assets: std::collections::HashSet<String> = gsd_security::prompts::ASSETS
        .iter()
        .flat_map(|(_, text)| {
            gsd_kit::prompt::render::without_notice(text)
                .lines()
                .map(|line| {
                    let line = gsd_kit::prompt::repoint::repoint(line, "claude-code", &|_| true);
                    gsd_kit::prompt::host::convert("claude-code", &line)
                        .trim()
                        .to_string()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let preamble = text.find("</specforge_gsd_preamble>").unwrap_or(0);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut at = 0;
    let starts: Vec<usize> = lines
        .iter()
        .map(|l| {
            let start = at;
            at += l.len() + 1;
            start
        })
        .collect();
    let hits = super::planning_names::hits("security-prompt:secure_phase", &text, |n| {
        let line = lines[n].trim();
        !(line.starts_with('|') && starts[n] < preamble) && !assets.contains(line)
    });
    super::planning_names::assert_none(&hits);
}

/// No rendered line of `secure_phase` tells an agent to edit a state file or a part of one, or to generate one: core's
/// detectors over the text as rendered, the preamble's file table (a read list) skipped.
#[test]
fn no_rendered_line_tells_an_agent_to_write_a_gsd_file() {
    use super::markdown_writes::{generates, section_edits};
    let text = text_of(&render(&model(), &[PHASE], "claude-code"));
    let preamble = text.find("</specforge_gsd_preamble>").unwrap_or(0);
    let preamble_lines = text[..preamble].matches('\n').count();
    let lines: Vec<&str> = text.split('\n').collect();
    let hits: Vec<String> = section_edits(&text)
        .union(&generates(&text))
        .filter(|n| !(lines[**n].trim().starts_with('|') && **n <= preamble_lines))
        .map(|n| {
            format!(
                "security-prompt:secure_phase:{} {}",
                n + 1,
                lines[*n].trim()
            )
        })
        .collect();
    assert!(
        hits.is_empty(),
        "a write is a typed edit (ADR 0343 D10):\n{}",
        hits.join("\n")
    );
}

/// The vault twin: phase 01's plans hold five closed threats and an open one.
fn vault() -> Project {
    super::harness::vault().with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

const W302: &str = "gsd.security/W302";
const W303: &str = "gsd.security/W303";
/// The vault's phase 01 audit.
const AUDIT: &str = "v1.0/01/01-SECURITY";

/// The closed threats of the vault's phase 01.
const CLOSED: [&str; 5] = [
    "v1.0/01/01-01/T-01-01",
    "v1.0/01/01-01/T-01-02",
    "v1.0/01/01-01/T-01-SC",
    "v1.0/01/01-02/T-01-03",
    "v1.0/01/01-02/T-01-SC",
];

/// The entities the rendered closing section lists as left to judge for `rule`, and the section's text.
fn left_to_judge(p: &Project, phase: &str, rule: &str) -> (Vec<String>, String) {
    let text = text_of(&render(p, &[phase], "claude-code"));
    let section = text
        .split("## The judged verdicts here (@gsd/security)")
        .nth(1)
        .and_then(|rest| rest.split("The template of those entities:").next())
        .unwrap_or_else(|| panic!("no judged section: {text}"))
        .to_string();
    let ids = section
        .lines()
        .filter_map(|l| {
            l.strip_prefix(&format!("- `{rule}` on `"))?
                .strip_suffix('`')
        })
        .map(String::from)
        .collect();
    (ids, section)
}

/// The closing section names the judged rules and their command, and lists every closed threat of the phase whose
/// `gsd.security/W302` verdict is not fresh and the phase's audit while its `W303` verdict is not; once each holds,
/// none is left.
#[test]
fn the_tail_lists_the_verdicts_of_the_phase_that_are_not_fresh() {
    let p = vault();
    let (ids, section) = left_to_judge(&p, PHASE, W302);
    assert_eq!(ids, CLOSED, "{section}");
    assert_eq!(left_to_judge(&p, PHASE, W303).0, [AUDIT], "{section}");
    assert!(
        section.contains("specforge query <entity> --inputs <rule> --format json"),
        "{section}"
    );
    assert!(
        section.contains("specforge gsd record-security-verdict <entity> <W302|W303> <outcome>"),
        "{section}"
    );
    assert_eq!(p.record(CLOSED[0], W302, "holds", &[]).code, 0);
    assert_eq!(left_to_judge(&p, PHASE, W302).0, &CLOSED[1..]);
    for threat in &CLOSED[1..] {
        let run = p.record(threat, W302, "holds", &[]);
        assert_eq!(run.code, 0, "{threat}: {}", run.all());
    }
    let (ids, section) = left_to_judge(&p, PHASE, W302);
    assert!(ids.is_empty(), "{section}");
    assert_eq!(left_to_judge(&p, PHASE, W303).0, [AUDIT], "{section}");
    assert_eq!(p.record(AUDIT, W303, "mapped", &[]).code, 0);
    let (ids, section) = left_to_judge(&p, PHASE, W303);
    assert!(ids.is_empty(), "{section}");
    assert!(
        section.contains("Every verdict this prompt judges is fresh: none is left to judge."),
        "{section}"
    );
}

/// A closed threat of another phase is never listed: the list follows the threat's plan to its phase.
#[test]
fn a_threat_of_another_phase_is_never_listed() {
    let p = vault();
    let threat = "v1.0/02/02-01/T-02-01";
    p.write(
        "spec/phase2-threats.spec",
        "threat T-02-01 \"Recipient key\" {\n  plan        v1.0/02/02-01\n  category    spoofing\n  component   \"src/share.js\"\n  severity    high\n  disposition mitigate\n  mitigation  \"the recipient's key is checked\"\n  status      closed\n}\n",
    );
    p.record_harness(threat, "status", "\"closed\"");
    assert_eq!(left_to_judge(&p, PHASE, W302).0, CLOSED);
    assert_eq!(left_to_judge(&p, "v1.0/02", W302).0, [threat]);
}

/// The list follows the references as the kernel resolved them, not as they were written: a threat whose `plan` and an
/// audit whose `phase` are written in a short form are the phase's as much as those written as full paths.
#[test]
fn a_reference_written_in_a_short_form_is_followed_to_its_phase() {
    let p = vault();
    p.edit(
        "spec/plans.spec",
        "threat T-01-01 \"Seal under a key that stays off the disk\" {\n  plan        v1.0/01/01-01",
        "threat T-01-01 \"Seal under a key that stays off the disk\" {\n  plan        01-01",
    );
    p.edit(
        "spec/security.spec",
        "security_audit 01-SECURITY \"Phase 1 security\" {\n  phase v1.0/01",
        "security_audit 01-SECURITY \"Phase 1 security\" {\n  phase 01",
    );
    let (ids, section) = left_to_judge(&p, PHASE, W302);
    assert_eq!(ids, CLOSED, "{section}");
    assert_eq!(left_to_judge(&p, PHASE, W303).0, [AUDIT], "{section}");
}

/// With `workflow.security_enforcement` off the host evaluates neither rule, so no verdict is owed and none is listed
/// as left to judge, though none is recorded.
#[test]
fn enforcement_off_leaves_nothing_to_judge() {
    let p = vault();
    p.settings_of(
        "@gsd/security",
        json!({"workflow.security_enforcement": false}),
    );
    for rule in [W302, W303] {
        let (ids, section) = left_to_judge(&p, PHASE, rule);
        assert!(ids.is_empty(), "{rule}: {section}");
        assert!(
            section.contains(
                "`workflow.security_enforcement` is off: neither rule is evaluated and no verdict is owed"
            ),
            "{section}"
        );
    }
}
