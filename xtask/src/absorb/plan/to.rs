//! Where a `To` home writes a value (ADR 0360 D2): each transform the table names, from a value written as it is to
//! a bound on its text's last line, and the helpers only they use. Only the phase's three ways write a reference,
//! through the id map ([`super::roadmap`], ADR 0372 D4).

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;

use super::super::how;
use super::super::ids;
use super::super::ledger::Why;
use super::super::read::OldEntity;
use super::super::table::{self, Row};
use super::Planner;

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
            // The id map assigned the id from this value before any draft.
            How::Id => {}
            How::Parent => self.parent(e, row.field, field, value),
            How::Requirements => self.requirements(e, row.field, field, value),
            How::Earlier => self.earlier(e, row.field, field, value),
            How::Category => {
                if value.as_str().is_some_and(|word| !word.trim().is_empty()) {
                    let category = ids::category(e);
                    if let Some(draft) = self.own(e) {
                        draft.set(field, Input::One(category));
                    }
                }
            }
            How::FeatureTier => {
                let status = value.as_str().unwrap_or(how::FEATURE_DEFAULT);
                self.tier(e, field, status);
            }
            How::Text => {
                if let Some(text) = value.as_str()
                    && let Some(draft) = self.own(e)
                {
                    draft.join(field, text.to_string());
                }
            }
            How::Bound => {
                if let Some(bound) = value.as_str()
                    && let Some(draft) = self.own(e)
                {
                    draft.bound(field, bound);
                }
            }
            How::Settled => {
                if let Some(input) = how::settled(value)
                    && let Some(draft) = self.own(e)
                {
                    draft.set(field, input);
                }
            }
        }
    }

    /// `items` appended to the list `field`; a scheme reference among them is ledgered `dropped` (see
    /// [`how::SCHEME_REF`], ADR 0372 D8).
    fn list(&mut self, e: &OldEntity, old_field: &str, field: &str, items: Vec<String>) {
        let (refs, items): (Vec<String>, Vec<String>) =
            items.into_iter().partition(|item| how::is_scheme_ref(item));
        for item in refs {
            let why = Why::Dropped(how::SCHEME_REF.to_string());
            self.ledger
                .record(&e.kind, &e.id, old_field, json!(item), why);
        }
        if let Some(draft) = self.own(e) {
            draft.append(field, items);
        }
    }

    /// A feature's tier from its status; a deprecated feature with no reason is given one. A `v1` tier no written
    /// phase lists is `v2`, ledgered `filled` (ADR 0372 D7).
    fn tier(&mut self, e: &OldEntity, field: &str, status: &str) {
        let by_status = how::feature_tier(status);
        let tier = self.unlisted_tier(e, by_status);
        let unexplained = tier == "out_of_scope" && !e.fields.contains_key("reason");
        if tier != by_status {
            let why = Why::Filled {
                field: "requirement.tier".to_string(),
                value: json!(tier),
            };
            self.ledger.record(&e.kind, &e.id, field, Value::Null, why);
        }
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
