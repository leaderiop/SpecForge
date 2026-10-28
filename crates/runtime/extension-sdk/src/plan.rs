//! Building an edit plan (ADR 0289). Every op carries the revision of the file it changes, which the builder
//! takes from the input the call received: the entity's `file`, then `revisions[file]`. An extension never spells
//! one. An op on an entity the input does not hold gets an empty revision, which the host refuses `invalid_plan`
//! naming the op; a `create` in a file the input does not hold gets `absent`.

use std::collections::BTreeMap;

use serde_json::Value;
use specforge_protocol_types::{Cite, CollectInput, EditPlan, PlanObligation, PlanOp};

use crate::CommandGraph;

/// A judged rule's verdict, as `set_verdict` writes it (ADR 0289). It has no file revision: its `basis` is.
#[derive(Debug, Clone, Default)]
pub struct Verdict {
    pub entity: String,
    pub rule: String,
    pub outcome: String,
    pub judge: String,
    pub model: String,
    pub findings: Vec<Value>,
    pub cited: Vec<Cite>,
    pub values: BTreeMap<String, Value>,
    pub basis: BTreeMap<String, String>,
}

/// Builds the [`EditPlan`] a writing command or collector answers. Ops apply in the order they are added.
#[derive(Debug, Clone)]
pub struct PlanBuilder {
    /// Entity id to the file declaring it, from the graph the call received.
    files: BTreeMap<String, String>,
    revisions: BTreeMap<String, String>,
    plan: EditPlan,
}

impl PlanBuilder {
    /// A builder over `files` (entity id to its file) and the call's `revisions`.
    pub fn new(files: BTreeMap<String, String>, revisions: BTreeMap<String, String>) -> Self {
        PlanBuilder {
            files,
            revisions,
            plan: EditPlan::default(),
        }
    }

    pub(crate) fn of_graph(graph: &CommandGraph, revisions: &BTreeMap<String, String>) -> Self {
        let files = graph
            .nodes()
            .iter()
            .filter_map(|n| Some((n.id.clone(), n.file.clone()?)))
            .collect();
        Self::new(files, revisions.clone())
    }

    /// A builder for a collector's call: its graph and revisions.
    pub fn for_collect(input: &CollectInput) -> Self {
        let files = input
            .graph
            .iter()
            .flat_map(|g| &g.nodes)
            .filter_map(|n| Some((n.id.clone(), n.file.clone()?)))
            .collect();
        Self::new(files, input.revisions.clone())
    }

    /// The line the CLI prints above the diff.
    pub fn title(&mut self, title: &str) -> &mut Self {
        self.plan.title = Some(title.to_string());
        self
    }

    fn revision_of(&self, entity: &str) -> String {
        self.files
            .get(entity)
            .and_then(|file| self.revisions.get(file))
            .cloned()
            .unwrap_or_default()
    }

    fn push(&mut self, op: PlanOp) -> &mut Self {
        self.plan.ops.push(op);
        self
    }

    /// Create `id` of `kind` in `file`, with `fields` and `verify` statements; later ops may name it.
    pub fn create(
        &mut self,
        kind: &str,
        id: &str,
        title: &str,
        file: &str,
        fields: BTreeMap<String, Value>,
        verify: Vec<PlanObligation>,
    ) -> &mut Self {
        let revision = self
            .revisions
            .get(file)
            .cloned()
            .unwrap_or_else(|| "absent".to_string());
        self.files.insert(id.to_string(), file.to_string());
        self.revisions
            .entry(file.to_string())
            .or_insert_with(|| revision.clone());
        self.push(PlanOp::Create {
            kind: kind.to_string(),
            id: id.to_string(),
            title: title.to_string(),
            file: file.to_string(),
            revision,
            fields,
            verify,
        })
    }

    /// Set `field` (a path: `status`, `completion.at`, `checks[0].status`) to `value`.
    pub fn set(&mut self, entity: &str, field: &str, value: impl Into<Value>) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Set {
            entity: entity.to_string(),
            field: field.to_string(),
            value: value.into(),
            revision,
        })
    }

    pub fn unset(&mut self, entity: &str, field: &str) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Unset {
            entity: entity.to_string(),
            field: field.to_string(),
            revision,
        })
    }

    /// Append `value` to the list `field`.
    pub fn append(&mut self, entity: &str, field: &str, value: impl Into<Value>) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Append {
            entity: entity.to_string(),
            field: field.to_string(),
            value: value.into(),
            revision,
        })
    }

    /// Remove the `index`th item of the list `field`.
    pub fn remove(&mut self, entity: &str, field: &str, index: usize) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Remove {
            entity: entity.to_string(),
            field: field.to_string(),
            index,
            revision,
        })
    }

    pub fn link(&mut self, entity: &str, field: &str, target: &str) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Link {
            entity: entity.to_string(),
            field: field.to_string(),
            target: target.to_string(),
            revision,
        })
    }

    pub fn unlink(&mut self, entity: &str, field: &str, target: &str) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Unlink {
            entity: entity.to_string(),
            field: field.to_string(),
            target: target.to_string(),
            revision,
        })
    }

    pub fn retitle(&mut self, entity: &str, title: &str) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Retitle {
            entity: entity.to_string(),
            title: title.to_string(),
            revision,
        })
    }

    pub fn add_verify(&mut self, entity: &str, kind: Option<&str>, text: &str) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::AddVerify {
            entity: entity.to_string(),
            kind: kind.map(str::to_string),
            text: text.to_string(),
            revision,
        })
    }

    pub fn remove_verify(&mut self, entity: &str, kind: Option<&str>, text: &str) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::RemoveVerify {
            entity: entity.to_string(),
            kind: kind.map(str::to_string),
            text: text.to_string(),
            revision,
        })
    }

    /// Delete `entity`; with `unlink_references`, also the references to it.
    pub fn delete(&mut self, entity: &str, unlink_references: bool) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Delete {
            entity: entity.to_string(),
            revision,
            unlink_references,
        })
    }

    /// Rename `entity` to `to`: the declaration and every reference.
    pub fn rename(&mut self, entity: &str, to: &str) -> &mut Self {
        let revision = self.revision_of(entity);
        self.push(PlanOp::Rename {
            entity: entity.to_string(),
            to: to.to_string(),
            revision,
        })
    }

    /// Write a judged rule's verdict (the host refuses it unless a judged rule is declared).
    pub fn set_verdict(&mut self, verdict: Verdict) -> &mut Self {
        self.push(PlanOp::SetVerdict {
            entity: verdict.entity,
            rule: verdict.rule,
            outcome: verdict.outcome,
            judge: verdict.judge,
            model: verdict.model,
            findings: verdict.findings,
            cited: verdict.cited,
            values: verdict.values,
            basis: verdict.basis,
        })
    }

    /// The plan, to answer with [`crate::CommandOutput::with_plan`].
    pub fn finish(&self) -> EditPlan {
        self.plan.clone()
    }
}
