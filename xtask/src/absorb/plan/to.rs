//! Where a `To` home writes a value (ADR 0360 D2): each transform the table names, from a value written as it is to
//! a decision status as its outcome, and the helpers only they use. No transform writes a reference (ADR 0370 D8).

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;
use std::collections::BTreeMap;

use super::super::how;
use super::super::ledger::Why;
use super::super::read::OldEntity;
use super::super::table::{self, NO_SUPERSESSION, Row};
use super::{Draft, Planner, solution_id};

impl Planner {
    pub(super) fn to(
        &mut self,
        e: &OldEntity,
        row: &Row,
        field: &str,
        how: table::How,
        value: &Value,
    ) {
        use table::How;
        match how {
            How::As => match how::as_written(value) {
                Input::Many(items) => self.list(e, row.field, field, items),
                other => {
                    if let Some(draft) = self.own(e) {
                        draft.set(field, other);
                    }
                }
            },
            How::Words => self.list(e, row.field, field, how::items(value)),
            How::Solution => self.solution(e, field, value),
            How::FeatureTier => {
                let status = value.as_str().unwrap_or(how::FEATURE_DEFAULT);
                self.tier(e, field, status);
            }
            How::Text => {
                if let (Some(text), Some(draft)) = (value.as_str(), self.drafts.get_mut(&e.id)) {
                    draft.join(field, text.to_string());
                }
            }
            How::Outcome if value.as_str() == Some("superseded") => {
                let why = Why::Dropped(NO_SUPERSESSION.to_string());
                self.ledger
                    .record(&e.kind, &e.id, row.field, value.clone(), why);
            }
            How::Outcome | How::Settled => {
                if let Some(input) = how::word(how, value)
                    && let Some(draft) = self.own(e)
                {
                    draft.set(field, input);
                }
            }
        }
    }

    /// `items` appended to the list `field`; a scheme reference among them is ledgered (see [`how::SCHEME_REF`]).
    fn list(&mut self, e: &OldEntity, old_field: &str, field: &str, items: Vec<String>) {
        let (refs, items): (Vec<String>, Vec<String>) =
            items.into_iter().partition(|item| how::is_scheme_ref(item));
        for item in refs {
            self.unconverted(e, old_field, json!(item), how::SCHEME_REF);
        }
        if let Some(draft) = self.own(e) {
            draft.append(field, items);
        }
    }

    /// A feature's solution as a decision of the project's own, beside the feature's requirement.
    fn solution(&mut self, e: &OldEntity, field: &str, value: &Value) {
        let Some(own) = self.drafts.get(&e.id) else {
            return;
        };
        let id = solution_id(&e.id);
        let draft = Draft {
            kind: "decision",
            id: id.clone(),
            title: own.title.clone(),
            file: own.file.clone(),
            fields: BTreeMap::from([
                (field.to_string(), how::as_written(value)),
                ("origin".to_string(), Input::One("project".to_string())),
            ]),
        };
        self.add_after(&e.id, draft);
        let why = Why::Created {
            kind: "decision".to_string(),
            id,
        };
        self.ledger
            .record(&e.kind, &e.id, "solution", value.clone(), why);
    }

    /// A feature's tier from its status; a deprecated feature with no reason is given one.
    fn tier(&mut self, e: &OldEntity, field: &str, status: &str) {
        let tier = how::feature_tier(status);
        let unexplained = tier == "out_of_scope" && !e.fields.contains_key("reason");
        let Some(draft) = self.own(e) else {
            return;
        };
        draft.set(field, Input::One(tier.to_string()));
        if unexplained {
            draft.set("reason", Input::One(status.to_string()));
            let why = Why::Filled {
                field: "requirement.reason".to_string(),
                value: json!(status),
            };
            self.ledger
                .record(&e.kind, &e.id, "reason", Value::Null, why);
        }
    }
}
