//! How the editor words a judged rule's standing (ADR 0290): the hover's Verdicts rows and the code lenses' titles.

use specforge_graph::VerdictJson;
use specforge_protocol_types::Judgement;

/// The verdict `judgement` stands on, as the entity's statements wrote it.
fn written<'a>(judgement: &Judgement, verdicts: &'a [VerdictJson]) -> Option<&'a VerdictJson> {
    verdicts.iter().find(|v| v.rule == judgement.rule)
}

/// How many findings `verdict` lists.
fn findings(verdict: Option<&VerdictJson>) -> usize {
    verdict
        .and_then(|v| v.members.get("findings"))
        .and_then(|f| f.as_value().as_array().map(Vec::len))
        .unwrap_or(0)
}

/// The Verdicts row: the rule, then where it stands, what changed and who judged.
pub(crate) fn row(judgement: &Judgement, verdicts: &[VerdictJson]) -> String {
    let rule = format!("`{}`", judgement.rule);
    let verdict = written(judgement, verdicts);
    let outcome = judgement.outcome.as_deref().unwrap_or("?");
    match judgement.state.as_str() {
        "fresh" => {
            let at = judgement.at.as_deref().map(|at| format!(" · {at}"));
            let judge = verdict
                .and_then(|v| v.text("judge"))
                .map(|judge| format!(" · {judge}"));
            let status = if judgement.passed {
                "fresh".to_string()
            } else {
                format!("failed — {} findings", findings(verdict))
            };
            format!(
                "{rule} · {outcome} · {status}{}{}",
                at.unwrap_or_default(),
                judge.unwrap_or_default()
            )
        }
        "stale" => format!("{rule} · stale — changed: {}", judgement.changed.join(", ")),
        "missing" => format!("{rule} · no verdict"),
        "unrecorded" => format!("{rule} · not recorded"),
        other => format!("{rule} · {other}"),
    }
}

/// The hover's section: one row per judged rule that judges the entity; `None` when none does.
pub(crate) fn section(judgements: &[Judgement], verdicts: &[VerdictJson]) -> Option<String> {
    (!judgements.is_empty()).then(|| {
        let rows: Vec<String> = judgements
            .iter()
            .map(|j| format!("- {}", row(j, verdicts)))
            .collect();
        format!("**Verdicts**\n{}", rows.join("\n"))
    })
}

/// The title of the code lens at a verdict's rule token.
pub(crate) fn lens(judgement: &Judgement, verdicts: &[VerdictJson]) -> String {
    let rule = &judgement.rule;
    match judgement.state.as_str() {
        "fresh" if judgement.passed => format!("{rule}: fresh, passed"),
        "fresh" => format!(
            "{rule}: failed ({} findings)",
            findings(written(judgement, verdicts))
        ),
        "stale" => format!("{rule}: stale ({})", judgement.changed.join(", ")),
        "unrecorded" => format!("{rule}: not recorded"),
        other => format!("{rule}: {other}"),
    }
}

/// The title of the lens at the entity's name when `missing` rules have no verdict.
pub(crate) fn missing(missing: usize) -> Option<String> {
    (missing > 0).then(|| {
        format!(
            "{missing} verdict{} missing",
            if missing == 1 { "" } else { "s" }
        )
    })
}
