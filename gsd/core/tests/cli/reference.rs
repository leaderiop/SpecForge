//! `specforge gsd reference <file>`: the ported references served to a model (D-r42-14).

use super::commands_support::{gsd, json, refused, twin};

#[test]
fn reference_serves_an_embedded_reference() {
    let p = twin();
    let run = gsd(&p, "reference", &["references/verification-patterns.md"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let upstream = include_str!(
        "../../../parity/upstream/v1.18.0/gsd-core/references/verification-patterns.md"
    );
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
