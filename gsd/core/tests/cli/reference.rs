//! `specforge gsd reference <file>`: the ported references served to a model (D-r42-14).

use super::commands_support::{gsd, json, refused, twin};

#[test]
fn reference_serves_an_embedded_reference() {
    let p = twin();
    let run = gsd(&p, "reference", &["references/verification-patterns.md"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let upstream = include_str!(gsd_kit::snapshot!(
        "gsd-core/references/verification-patterns.md"
    ));
    assert!(
        run.stdout.contains("# Verification Patterns"),
        "{}",
        run.stdout
    );
    assert!(
        !run.stdout.contains("Ported from"),
        "the attribution line is not served"
    );
    assert!(!run.stdout.contains("~/.claude/gsd-core"), "{}", run.stdout);
    assert!(
        run.stdout.contains("specforge gsd reference gsd-core/"),
        "an @-line is re-pointed: {}",
        run.stdout
    );
    assert!(
        upstream.contains("~/.claude/gsd-core"),
        "the vendored text is upstream's"
    );
}

#[test]
fn reference_answers_json_and_accepts_the_full_path() {
    let p = twin();
    let run = gsd(
        &p,
        "reference",
        &[
            "gsd-core/references/verification-patterns.md",
            "--format",
            "json",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let answer = json(&run);
    assert_eq!(
        answer["path"],
        "gsd-core/references/verification-patterns.md"
    );
    assert!(answer["text"].as_str().is_some_and(|t| t.len() > 1000));
}

#[test]
fn reference_refuses_an_unknown_path_and_names_the_nearest() {
    let p = twin();
    let run = gsd(&p, "reference", &["references/verification-pattern.md"]);
    refused(
        &run,
        "invalid_input",
        &["the nearest is 'gsd-core/references/verification-patterns.md'"],
    );
}

#[test]
fn reference_names_a_templates_kind() {
    let p = twin();
    let run = gsd(&p, "reference", &["templates/roadmap.md"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let first = run.stdout.lines().next().unwrap_or_default();
    assert!(
        first.starts_with("SpecForge keeps this as `milestone` and `phase` entities"),
        "{first}"
    );
    let state = gsd(&p, "reference", &["templates/state.md"]);
    assert!(
        state.stdout.starts_with("SpecForge computes this"),
        "{}",
        state.stdout
    );
}

#[test]
fn a_reference_reads_the_setting_it_names() {
    let p = twin();
    let read = |p: &super::harness::Project| {
        gsd(p, "reference", &["references/compact-content-gate.md"]).stdout
    };
    let default = read(&p);
    assert!(default.contains("COMPACT_CONTENT=\"false\""), "{default}");
    p.settings(serde_json::json!({ "workflow.compact_content": true }));
    let set = read(&p);
    assert!(set.contains("COMPACT_CONTENT=\"true\""), "{set}");
    assert!(!set.contains("{{setting:"), "an unfilled token");
}

/// REVIEW-r42 N2: a file read through `reference` is converted for the host that asks, like the inlined text.
#[test]
fn reference_converts_the_text_for_the_host() {
    let p = twin();
    let read = |file: &str, host: &str| {
        let run = gsd(&p, "reference", &[file, "--host", host]);
        assert_eq!(run.code, 0, "{}", run.all());
        run.stdout
    };
    let claude = read("workflows/discuss-phase.md", "claude-code");
    assert!(!claude.contains("/gsd:"), "claude-code keeps a colon name");
    assert!(
        claude.contains("/gsd-plan-phase"),
        "claude-code has no hyphen name"
    );
    let codex = read("workflows/discuss-phase.md", "codex");
    assert!(
        !codex.contains("/gsd:") && !codex.contains("/gsd-plan-phase"),
        "codex keeps a slash name"
    );
    assert!(codex.contains("$gsd-"), "codex has no $gsd- name");
    let cursor = read("workflows/discuss-phase.md", "cursor");
    assert!(
        !cursor.contains("/gsd:") && !cursor.contains("AskUserQuestion"),
        "cursor keeps a Claude name"
    );
    assert!(
        cursor.contains("/gsd-plan-phase"),
        "cursor has no hyphen name"
    );
    // the nested references carry the host on, and the `@host` fallback is the host's runtime, not "claude"
    assert!(
        codex.contains("specforge gsd reference gsd-core/") && codex.contains("--host codex"),
        "{codex}"
    );
    for (host, runtime) in [
        ("claude-code", "claude"),
        ("codex", "codex"),
        ("cursor", "cursor"),
    ] {
        let text = read("workflows/smart-entry.md", host);
        assert!(
            text.contains(&format!("RUNTIME=\"{runtime}\"")),
            "{host}: {text}"
        );
    }
    let run = gsd(
        &p,
        "reference",
        &["workflows/smart-entry.md", "--host", "vim"],
    );
    assert_ne!(run.code, 0, "{}", run.all());
}
