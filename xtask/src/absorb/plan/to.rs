//! Where a `To` home writes a value (ADR 0360 D2): each transform the table names, from a value written as it is to
//! a metric read with the prove pass's reader, and the helpers only they use.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;
use std::collections::{BTreeMap, BTreeSet};

use super::super::how;
use super::super::ledger::Why;
use super::super::proof::Proved;
use super::super::read::OldEntity;
use super::super::table::{self, Row, VERIFY};
use super::{Draft, Planner, solution_id};

impl Planner<'_> {
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
            How::Ref => {
                if let Some(id) = value.as_str()
                    && let Some(id) = self.resolve(e, row.field, id)
                    && let Some(draft) = self.own(e)
                {
                    draft.set(field, Input::One(id));
                }
            }
            How::Steps => {
                let steps: Vec<BTreeMap<String, Input>> = how::items(value)
                    .into_iter()
                    .map(|action| BTreeMap::from([("action".to_string(), Input::One(action))]))
                    .collect();
                if let (false, Some(draft)) = (steps.is_empty(), self.drafts.get_mut(&e.id)) {
                    draft.set(field, Input::Blocks(steps));
                }
            }
            How::Solution => self.solution(e, field, value),
            How::Proof => {
                let text = value["text"].as_str().unwrap_or_default();
                match self.proofs.of(&e.id, text) {
                    Proved::Tests(tests) => {
                        self.cited.extend(tests.iter().cloned());
                        if let Some(draft) = self.own(e) {
                            draft.append(field, tests);
                        }
                    }
                    Proved::Unproven(tests) => {
                        let why = Why::Unproven { tests };
                        self.ledger
                            .record(&e.kind, &e.id, VERIFY, value.clone(), why);
                    }
                }
            }
            How::FeatureTier => {
                let status = value.as_str().unwrap_or(how::FEATURE_DEFAULT);
                self.tier(e, field, status);
            }
            How::Text => {
                if let (Some(text), Some(draft)) = (value.as_str(), self.drafts.get_mut(&e.id)) {
                    draft.join(field, text.to_string());
                }
            }
            How::Refs => {
                let ids: Vec<String> = how::items(value)
                    .into_iter()
                    .filter_map(|id| self.resolve(e, row.field, &id))
                    .collect();
                if let Some(draft) = self.own(e) {
                    draft.append(field, ids);
                }
            }
            How::InvertOnto => {
                for id in how::items(value) {
                    if let Some(target) = self.resolve(e, row.field, &id)
                        && let Some(draft) = self.drafts.get_mut(&target)
                    {
                        draft.append(field, vec![e.id.clone()]);
                        draft.inverted.insert(field.to_string());
                    }
                }
            }
            How::Outcome | How::Settled | How::Options => {
                if let Some(input) = how::word(how, value)
                    && let Some(draft) = self.own(e)
                {
                    draft.set(field, input);
                }
            }
            How::Category | How::Priority => {
                let word = value.as_str().unwrap_or_default();
                let mapped = if how == How::Category {
                    how::category(word)
                } else {
                    how::priority(word)
                };
                let written = match mapped {
                    how::Mapped::Same(word) => word,
                    how::Mapped::Collapsed(word) => {
                        self.collapsed(e, row.field, value, field, word);
                        word
                    }
                    how::Mapped::Unknown(why) => {
                        return self.unconverted(e, row.field, value.clone(), &why);
                    }
                };
                if let Some(draft) = self.own(e) {
                    draft.set(field, Input::One(written.to_string()));
                }
            }
            How::Sod(scale) => match how::rating(scale, value) {
                Ok(n) => {
                    if let Some(draft) = self.own(e) {
                        draft.set(field, how::number(n));
                    }
                }
                Err(why) => self.unconverted(e, row.field, value.clone(), &why),
            },
            How::Post => {
                for (member, rated) in value.as_object().into_iter().flatten() {
                    let scale = table::Scale::ALL.into_iter().find(|s| s.member() == member);
                    match scale.map(|scale| how::rating(scale, rated)) {
                        Some(Ok(n)) => {
                            if let Some(draft) = self.own(e) {
                                draft.member(field, member, how::number(n));
                            }
                        }
                        Some(Err(why)) => self.unconverted(e, row.field, value.clone(), &why),
                        // Its rpn is compared with the computed post_rpn.
                        None if member == "rpn" => {}
                        None => self.unconverted(
                            e,
                            row.field,
                            value.clone(),
                            &format!("the {member} member has no home in the post block"),
                        ),
                    }
                }
            }
            How::PostWord(scale) => {
                let member = scale.member();
                let block = e.fields.get("post_mitigation");
                if block.is_some_and(|block| block.get(member).is_some()) {
                    let why = format!("post_mitigation's {member} is written in its place");
                    return self.unconverted(e, row.field, value.clone(), &why);
                }
                match how::rating(scale, value) {
                    Ok(n) => {
                        if let Some(draft) = self.own(e) {
                            draft.member(field, member, how::number(n));
                        }
                    }
                    Err(why) => self.unconverted(e, row.field, value.clone(), &why),
                }
            }
            How::Bound => self.metric(e, field),
            // Read with the metric; alone, it joins the text.
            How::Threshold if e.fields.contains_key("metric") => {}
            How::Threshold => self.metric(e, field),
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

    /// A feature's solution as a decision of its own, affecting the feature's requirement.
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
                ("affects".to_string(), Input::Many(vec![e.id.clone()])),
            ]),
            inverted: BTreeSet::new(),
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

    /// The old metric and threshold as the `field` record and lines of the entity's `text`.
    fn metric(&mut self, e: &OldEntity, field: &str) {
        let metric = e.fields.get("metric");
        let threshold = e.fields.get("threshold");
        let read = how::metric(
            metric.and_then(Value::as_str).unwrap_or_default(),
            threshold.and_then(Value::as_str),
        );
        if let Some(value) = metric {
            if let Some(why) = &read.unconverted {
                self.unconverted(e, "metric", value.clone(), why);
            }
            if let Some(direction) = read.strict {
                let path = format!("{field}.direction");
                self.collapsed(e, "metric", value, &path, direction);
            }
        }
        if let (Some(value), false) = (threshold, read.threshold_taken) {
            let why = "no one-name metric it completes; it joins the text";
            self.unconverted(e, "threshold", value.clone(), why);
        }
        if let Some(draft) = self.drafts.get_mut(&e.id) {
            if let Some(record) = read.record {
                draft.set(field, Input::Entries(record));
            }
            for text in read.text {
                draft.join("text", text);
            }
        }
    }

    /// `value` of the old `field` written as the coarser `word` in the new `path` (a field of the entity's kind).
    pub(super) fn collapsed(
        &mut self,
        e: &OldEntity,
        field: &str,
        value: &Value,
        path: &str,
        word: &str,
    ) {
        let kind = table::new_kind(&e.kind).unwrap_or_default();
        self.ledger.record(
            &e.kind,
            &e.id,
            field,
            value.clone(),
            Why::Collapsed {
                field: format!("{kind}.{path}"),
                into: json!(word),
            },
        );
    }
}
