//! The `checks` collector: each checked value's outcome, published as a `check_outcome` fact (ADR 0327). It reads
//! only the rows the host serves it, so it runs when named (`collect --runner checks`) or under `checks.publish`.

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
            "When its oldest answer was observed (RFC 3339, UTC)",
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
    fact
}

/// Unix milliseconds as `YYYY-MM-DDTHH:MM:SSZ`.
pub(crate) fn rfc3339(millis: u64) -> String {
    let seconds = millis / 1000;
    let (days, of_day) = ((seconds / 86_400) as i64, seconds % 86_400);
    // The civil date of a day count from 1970-01-01 (H. Hinnant's `civil_from_days`).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let of_era = z - era * 146_097;
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * of_year + 2) / 153;
    let day = of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 {
        shifted + 3
    } else {
        shifted - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3_600,
        of_day % 3_600 / 60,
        of_day % 60
    )
}
