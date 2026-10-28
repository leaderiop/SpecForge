//! The host's judged rules (ADR 0290) behind the port `set_verdict` is checked through: the registry build's judged
//! rules, the digests of their inputs (`specforge_project::judged::inputs`, the one the inputs query shares) and the
//! verdict statement written into the entity's block.

use std::collections::BTreeMap;

use serde_json::Value;
use specforge_graph::Found;
use specforge_project::judged::inputs;
use specforge_protocol_types::Cite;
use specforge_registry::PlainDisk;

use crate::read::judged::cited_of;

use super::verdict::{JudgedRule, JudgedRules, UnknownCite, VerdictMember};
use crate::Refusal;
use crate::authoring::edit::{self, EditField, Site};
use crate::view::ProjectView;

/// The judged rules of one project, by the name a plan gives them: the qualified code, and the local one when no other
/// rule has it.
#[derive(Debug, Clone, Default)]
pub struct HostJudgedRules {
    rules: BTreeMap<String, (String, JudgedRule)>,
}

impl HostJudgedRules {
    /// The judged rules the project `view` describes declare.
    pub fn of(view: &ProjectView<'_>) -> Self {
        let registries = view.registries();
        let mut rules = BTreeMap::new();
        let mut local: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for rule in registries.judged_rules() {
            let version = registries
                .declaration(&rule.extension)
                .map(|d| d.handshake.version.clone())
                .unwrap_or_default();
            let mut kinds: Vec<String> = rule.targets.iter().map(|t| t.kind.clone()).collect();
            kinds.sort();
            let values = rule
                .values
                .iter()
                .filter_map(|v| v.split_once('.'))
                .filter(|(kind, _)| kinds.iter().any(|k| k == kind))
                .map(|(_, field)| field.to_string())
                .collect();
            let port = JudgedRule {
                extension: rule.extension.clone(),
                judge: rule.judge.clone(),
                version,
                targets: kinds,
                outcomes: rule.outcomes.clone(),
                values,
            };
            if let Some((_, short)) = rule.code.rsplit_once('/') {
                local
                    .entry(short.to_string())
                    .or_default()
                    .push(rule.code.clone());
            }
            rules.insert(rule.code.clone(), (rule.code.clone(), port));
        }
        for (short, codes) in local {
            if let [code] = codes.as_slice()
                && let Some(found) = rules.get(code).cloned()
            {
                rules.entry(short).or_insert(found);
            }
        }
        Self { rules }
    }
}

impl JudgedRules for HostJudgedRules {
    fn rule(&self, name: &str) -> Option<JudgedRule> {
        self.rules.get(name).map(|(_, rule)| rule.clone())
    }

    fn digests(
        &self,
        view: &ProjectView<'_>,
        entity: &str,
        rule: &str,
        cited: &[Cite],
    ) -> Result<BTreeMap<String, String>, UnknownCite> {
        let (Some((code, _)), Found::One(path)) = (self.rules.get(rule), view.graph().find(entity))
        else {
            return Ok(BTreeMap::new());
        };
        let Some(judged) = view.registries().judged_rule(code) else {
            return Ok(BTreeMap::new());
        };
        let shown = cited_of(view, cited).map_err(UnknownCite)?;
        // shortcut: file inputs are read from the plain file system, not through an MCP call's bound; a judged
        // input outside the bound is read anyway. Route it through the view's containment when a bound is set.
        Ok(
            inputs(view.project(), &PlainDisk, path.as_str(), judged, &shown)
                .map(|found| found.digests.into_iter().collect())
                .unwrap_or_default(),
        )
    }

    fn admits(
        &self,
        view: &ProjectView<'_>,
        kind: &str,
        field: &str,
        value: &Value,
    ) -> Result<(), String> {
        let Some((_, form)) = view.registries().fields().get_path(kind, field) else {
            return Err(format!("{kind} has no field '{field}'"));
        };
        form.admit_json(value).map(|_| ()).map_err(|refusal| {
            refusal
                .suggestion
                .unwrap_or_else(|| format!("the field is declared {}", form.field_type().as_str()))
        })
    }

    fn write(
        &self,
        site: &Site<'_>,
        text: &str,
        verdict: &VerdictMember,
    ) -> Result<String, Refusal<EditField>> {
        edit::verdict(site, text, &verdict.rule, &statement(verdict))
    }
}

/// The `verdict` statement for `member`, laid out as the formatter will keep it.
fn statement(member: &VerdictMember) -> String {
    let mut lines = vec![format!("verdict {} {} {{", member.rule, member.outcome)];
    let mut put = |text: String| lines.push(format!("  {text}"));
    put(format!("judge {}", quoted(&member.judge)));
    put(format!("model {}", quoted(&member.model)));
    put(format!("at {}", quoted(&member.at)));
    let (mut entities, mut files, mut tests) = (Vec::new(), Vec::new(), Vec::new());
    for cite in &member.cited {
        match cite {
            Cite::Entity { entity } => entities.push(entity.clone()),
            Cite::File { file } => files.push(quoted(file)),
            Cite::Test { test } => tests.push(quoted(test)),
        }
    }
    if !entities.is_empty() || !files.is_empty() {
        entities.extend(files);
        put(format!("cited [{}]", entities.join(", ")));
    }
    if !tests.is_empty() {
        put(format!("cited_tests [{}]", tests.join(", ")));
    }
    put("inputs {".to_string());
    for (name, digest) in &member.inputs {
        put(format!("  {name} {}", quoted(digest)));
    }
    put("}".to_string());
    if !member.findings.is_empty() {
        let items: Vec<String> = member.findings.iter().map(spec).collect();
        put(format!("findings [{}]", items.join(", ")));
    }
    if !member.values.is_empty() {
        let members: Vec<String> = member
            .values
            .iter()
            .map(|(key, value)| format!("{key} {}", spec(value)))
            .collect();
        put(format!("values {{ {} }}", members.join(" ")));
    }
    lines.push("}".to_string());
    lines.join("\n")
}

fn quoted(text: &str) -> String {
    specforge_graph::field_form::spell::quoted(text)
}

/// `value` as a spec value: a string quoted, a number or boolean bare, a list in brackets, an object a block.
fn spec(value: &Value) -> String {
    match value {
        Value::String(text) => quoted(text),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(spec).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(members) => format!(
            "{{ {} }}",
            members
                .iter()
                .map(|(key, value)| format!("{key} {}", spec(value)))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        other => other.to_string(),
    }
}
