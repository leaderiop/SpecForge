//! The `checks` collector: each checked value's outcome, published as a `check_outcome` fact (ADR 0327). It reads
//! only the rows the host serves it, so it runs when named (`collect --runner checks`) or under `checks.publish`.

use jiff::Timestamp;
use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::{CheckOutcome, CheckStatus};

pub(crate) const KIND: &str = "check_outcome";
const SCHEME: &str = "check";
const RESULT: &str = "status";
const STATUS: Vocabulary = Vocabulary::new(&["passed", "failed", "skipped"]);

pub(crate) fn contribute(c: &mut ContributionsBuilder) {
    c.kind(KIND, |k| {
        k.description(
            "What one check said of one value (ADR 0336), written by the checks collector: \
             check:<entity>/<field>#<item>@<checker>",
        )
        .collected(&[SCHEME])
        .result(RESULT);
        let text = |k: &mut KindBuilder, name: &str, description: &str| {
            k.field(name, |f| {
                f.field_type(FieldType::String).description(description);
            });
        };
        text(k, "entity", "The id of the checked entity");
        text(k, "field", "The checked field's declared path");
        k.field("item", |f| {
            f.field_type(FieldType::Integer)
                .description("The value's place in its list; 0 for a single value");
        });
        text(
            k,
            "checker",
            "The checker, qualified: @specforge/checks:url",
        );
        k.field(RESULT, |f| {
            f.vocabulary(&STATUS)
                .description("passed (no finding), failed (a finding) or skipped (not performed)");
        });
        text(
            k,
            "outcome",
            "What its probes answered: ok, status:404, unreachable:dns, …",
        );
        k.field("codes", |f| {
            f.field_type(FieldType::StringList)
                .description("The codes it found");
        });
        text(
            k,
            "observed_at",
            "When its oldest journaled answer was observed (RFC 3339, UTC): the journal record's; absent when it read only the disk or git",
        );
        text(
            k,
            "digest",
            "The digest of what its probes answered, free of when (sha256:<hex>)",
        );
    });
    c.collector("checks", |k| {
        k.input(Input::CheckOutcomes).collect(collect);
    });
}

/// One fact per served row.
pub(crate) fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let facts = input.check_outcomes.iter().map(fact).collect();
    Ok(CollectOutput {
        facts,
        ..CollectOutput::default()
    })
}

pub(crate) fn fact(row: &CheckOutcome) -> Fact {
    let id = format!(
        "{SCHEME}:{}/{}#{}@{}",
        row.entity, row.field, row.item, row.checker
    );
    let status = match row.status {
        CheckStatus::Passed => "passed",
        CheckStatus::Failed => "failed",
        CheckStatus::NotRun => "skipped",
    };
    let mut fact = Fact::new(KIND, id)
        .field("entity", row.entity.as_str())
        .field("field", row.field.as_str())
        .field("item", row.item)
        .field("checker", row.checker.as_str())
        .field(RESULT, status);
    if let Some(outcome) = &row.outcome {
        fact = fact.field("outcome", outcome.to_string());
    }
    if !row.codes.is_empty() {
        fact = fact.field("codes", row.codes.clone());
    }
    if let Some(at) = row.observed_at {
        fact = fact.field("observed_at", rfc3339(at));
    }
    if let Some(digest) = &row.digest {
        fact = fact.field("digest", digest.as_str());
    }
    fact
}

/// Unix milliseconds as `YYYY-MM-DDTHH:MM:SSZ`, to the second; a time past what RFC 3339 writes as itself.
pub(crate) fn rfc3339(millis: u64) -> String {
    let seconds = i64::try_from(millis / 1000).unwrap_or(i64::MAX);
    match Timestamp::from_second(seconds) {
        Ok(at) => at.to_string(),
        Err(_) => millis.to_string(),
    }
}
