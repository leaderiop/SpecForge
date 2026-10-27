//! Typed edits (ADR 0151): create, change and delete entities in a project's `.spec` files. Each value is written in
//! the form its field's declaration decides, each touched block is laid out by the formatter, and nothing is
//! written unless the edited project reports no error it did not report before (E080) and its graph changes only
//! what the edits declare (ADR 0198). The edits are written as
//! one project change (ADR 0128).

mod block;
mod create;
mod declared;
mod defaults;
mod keep;
mod outcome;
mod path;
mod skeleton;
pub(crate) mod validate;
pub mod write;

pub use outcome::{EditDocument, Edited, run};
pub use skeleton::{Skeleton, Skeletons, Slot, SlotOf};

use crate::authoring::format::{Configs, Forms, Place};
use crate::authoring::planned::PlannedChange;
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind, Refusal};
use serde::{Deserialize, Serialize};
use specforge_common::DiagnosticList;
use specforge_common::Sym;
use specforge_common::shape::{Definitions, Shape};
use specforge_formatter::FormatConfig;
use specforge_graph::{Found, Scopes};
use specforge_parser::LineBreak;
use std::collections::BTreeMap;

/// One typed edit. The MCP `specforge.edit` tool reads a list of them (`op` names the variant); each CLI verb
/// builds one. Every edit of a member (set, unset, link, unlink, add or remove a verify statement) refuses a union
/// (`not_editable`); retitle and delete take one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Shape)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    /// A new entity in `file` (relative to the project root; created when missing).
    Create {
        kind: String,
        id: String,
        title: String,
        file: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        fields: BTreeMap<String, Input>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        verify: Vec<Obligation>,
    },
    /// `field` (or `field.entry` inside a block field) holds `value`; `annotations` (`readonly`, or
    /// `name=value`, written `@name "value"`)
    /// replace the field's when given.
    Set {
        entity_id: String,
        field: String,
        value: Input,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        annotations: Option<Vec<String>>,
    },
    /// `field` (or `field.entry`) holds none afterwards: removed, or, over a file default, written `[]` (a list) or
    /// `{}` (a block field); refused for a field of one value with a default. A second unset changes nothing.
    Unset { entity_id: String, field: String },
    /// The header's title becomes `title`; a blank one is refused (`invalid_title`).
    Retitle { entity_id: String, title: String },
    /// `target` is added to the list `field`. No change when it is already there (own or inherited).
    Link {
        entity_id: String,
        field: String,
        target: String,
    },
    /// `target` is removed from the list `field`. No change when it is absent.
    Unlink {
        entity_id: String,
        field: String,
        target: String,
    },
    /// A `verify [kind] "text"` line is added (after the last one). No change when an equal one is there.
    AddVerify {
        entity_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        text: String,
    },
    /// The `verify` line with this kind and text is removed. No change when there is none.
    RemoveVerify {
        entity_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        text: String,
    },
    /// The entity's block (and its doc comment, never the format header) is removed. With `unlink_references`, every
    /// reference to it is removed in the same change (an unlink, or an unset for a single `reference` field).
    Delete {
        entity_id: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        unlink_references: bool,
    },
}

/// A value as a caller gives it; the field's declaration decides how it is written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Input {
    One(String),
    Many(Vec<String>),
    Entries(BTreeMap<String, String>),
}

impl Shape for Input {
    fn body(defs: &mut Definitions) -> serde_json::Value {
        serde_json::json!({"oneOf": [
            String::schema(defs),
            Vec::<String>::schema(defs),
            BTreeMap::<String, String>::schema(defs),
        ]})
    }
}

/// A `verify` obligation: given to `Create`, or named by the CLI's `--verify` text (ADR 0237).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Shape)]
pub struct Obligation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub text: String,
}

impl Obligation {
    /// `kind:text` as an obligation of an entity whose kind accepts `verify_kinds` (ADR 0237): the part before the
    /// first colon is the kind when it is one of them, the text what follows it with its leading spaces trimmed;
    /// otherwise the whole text and no kind.
    pub fn parse(text: &str, verify_kinds: &[String]) -> Obligation {
        match text.split_once(':') {
            Some((kind, rest)) if verify_kinds.iter().any(|k| k == kind) => Obligation {
                kind: Some(kind.to_string()),
                text: rest.trim_start().to_string(),
            },
            _ => Obligation {
                kind: None,
                text: text.to_string(),
            },
        }
    }

    /// The reading `parse` did not make of a word prefix: the whole text when it took a kind, the word as the kind
    /// when it kept the text; none without a word before the first colon.
    fn other(&self, text: &str) -> Option<Obligation> {
        match (&self.kind, text.split_once(':')) {
            (Some(_), _) => Some(Obligation {
                kind: None,
                text: text.to_string(),
            }),
            (None, Some((word, rest))) if specforge_graph::field_form::is_word(word) => {
                Some(Obligation {
                    kind: Some(word.to_string()),
                    text: rest.trim_start().to_string(),
                })
            }
            _ => None,
        }
    }

    /// The obligation `text` names on `entity_id`, as `--verify` gives it (ADR 0237): when the entity writes the
    /// other reading of a word prefix (as a kind, or kept in the text) and not the one [`Obligation::parse`] makes
    /// with its kind's verify kinds, that other one; otherwise the parse. An entity the project does not hold reads
    /// with no verify kinds, and the plan then refuses it (`not_found`) as it does today.
    pub fn on(view: &ProjectView<'_>, entity_id: &str, text: &str) -> Obligation {
        let path = match view.graph().find(entity_id) {
            Found::One(path) => path.to_string(),
            _ => entity_id.to_string(),
        };
        let node = view.graph().node(&path);
        let kinds = node.map_or(&[][..], |n| verify_kinds_of(view, n.kind.raw.as_str()));
        let parsed = Obligation::parse(text, kinds);
        let writes = |o: &Obligation| {
            node.is_some_and(|n| {
                n.obligations
                    .iter()
                    .any(|s| s.kind == o.kind.as_deref().unwrap_or("") && s.description == o.text)
            })
        };
        match parsed.other(text) {
            Some(other) if !writes(&parsed) && writes(&other) => other,
            _ => parsed,
        }
    }
}

/// The verify kinds `kind` accepts: declared, or an enhancement's (`KindRegistryEntry::allowed_verify_kinds`); none
/// for a kind no loaded extension declares, or one that accepts no verify statement.
pub fn verify_kinds_of<'v>(view: &ProjectView<'v>, kind: &str) -> &'v [String] {
    view.registries()
        .kinds()
        .get(kind)
        .map_or(&[], |entry| entry.allowed_verify_kinds.as_slice())
}

/// The part of a request a refusal is about (ADR 0072). MCP names every one `edits`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    Entity,
    Kind,
    Id,
    Title,
    Field,
    Value,
    Target,
    File,
}

/// A field a kind declares, as `specforge new` offers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: specforge_protocol_types::FieldType,
    pub description: String,
    /// Whether the kind requires the field.
    pub required: bool,
}

/// The fields `kind` declares, in the order a new block writes them (the kind's own, then its enhancements');
/// none for a kind no loaded extension declares.
pub fn fields_of(view: &ProjectView<'_>, kind: &str) -> Vec<FieldInfo> {
    block::order(view, kind)
        .into_iter()
        .filter_map(|name| {
            let entry = view.registries().fields().get(kind, &name)?;
            Some(FieldInfo {
                field_type: entry.field_type(),
                description: entry.declared().description.clone().unwrap_or_default(),
                required: entry.declared().required,
                name,
            })
        })
        .collect()
}

/// What a plan will write.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct EditReport {
    /// The entities whose block the edits, taken together, create, change or delete, sorted
    /// (`PlannedChange::entities`).
    pub entities: Vec<String>,
    /// The files the edits change, relative to the project root, sorted.
    pub affected_files: Vec<String>,
    /// A unified diff of every affected file (project-root-relative headers), in file order. Empty when nothing changes.
    pub diff: String,
    /// The warnings and infos the edited project reports that the project did not, in report order.
    pub added: DiagnosticList,
}

/// The files a list of edits changes, and what they would write.
#[derive(Debug, Clone)]
pub struct EditPlan {
    change: PlannedChange,
    report: EditReport,
}

impl EditPlan {
    pub fn report(&self) -> &EditReport {
        &self.report
    }

    /// What the plan writes (ADR 0184).
    pub fn change(&self) -> &PlannedChange {
        &self.change
    }
}

/// The part of the plan being made: the edited texts, and the project as the earlier edits leave it: who is in which
/// scope, and in which file an entity the plan created or moved is. Every edit reads these, not the graph, so it sees
/// what the edits before it made (ADR 0284). The texts are LF: each file is read in its `LineBreak` and written back
/// in it (ADR 0266).
struct Planner<'v> {
    view: &'v ProjectView<'v>,
    /// The plan's one configuration lookup, through the view's disk (ADR 0169).
    configs: Configs<'v>,
    texts: BTreeMap<String, String>,
    /// The scopes once the edits so far are made.
    scopes: Scopes,
    /// The file of each path an edit of the plan created, moved to or deleted (`None`); any other path is the
    /// graph's.
    files: BTreeMap<String, Option<String>>,
    declared: declared::Declared,
}

/// Where an entity is.
struct Located {
    key: String,
    kind: String,
    /// The entity's path (ADR 0284): what `id` named, spelled in full.
    path: String,
}

impl<'v> Planner<'v> {
    /// The format configuration of the file `key` names (relative to the project root).
    fn config_of(&mut self, key: &str) -> FormatConfig {
        let path = self.view.env().sources().path(key);
        self.configs.layout(Place::File(&path))
    }

    fn text(&self, key: &str) -> Option<String> {
        match self.texts.get(key) {
            Some(text) => Some(text.clone()),
            None => self
                .view
                .project()
                .source_text(key)
                .map(|t| LineBreak::of(&t).lf(&t).into_owned()),
        }
    }

    /// The file of the entity at `path` as the plan leaves it.
    fn file_of(&self, path: &str) -> Option<String> {
        match self.files.get(path) {
            Some(file) => file.clone(),
            None => self
                .view
                .graph()
                .node(path)
                .map(|node| node.source_span.file.to_string()),
        }
    }

    /// The path `id` names in the project as the plan leaves it: the entity by its path or a unique short form;
    /// refused when a short form names several. `id` itself when it names none (the caller refuses it).
    fn path_of(&self, id: &str) -> Result<String, Refusal<EditField>> {
        match self.scopes.resolve(None, id, &|_| true) {
            Found::One(path) => Ok(path.to_string()),
            Found::Ambiguous(paths) => {
                Err(crate::view::ambiguous(id, &paths).about(EditField::Entity))
            }
            Found::Mistyped(_) | Found::None(_) => Ok(id.to_string()),
        }
    }

    fn locate(&self, id: &str) -> Result<Located, Refusal<EditField>> {
        let path = self.path_of(id)?;
        match self.scopes.kind(&path).zip(self.file_of(&path)) {
            Some((kind, key)) => Ok(Located {
                key,
                kind: kind.to_string(),
                path,
            }),
            None => Err(crate::view::not_found(self.view.graph(), id).about(EditField::Entity)),
        }
    }

    /// `change` applied to the text of `id`'s file; the file and the entity are recorded when it changed it.
    fn on(
        &mut self,
        id: &str,
        change: impl FnOnce(&block::Site<'_>, &str) -> Result<String, Refusal<EditField>>,
    ) -> Result<(), Refusal<EditField>> {
        let located = self.locate(id)?;
        let text = self.text(&located.key).ok_or_else(|| {
            OpError::new(
                OpErrorKind::Internal,
                crate::FILE_UNREADABLE,
                format!("no text was read for {}", located.key),
            )
        })?;
        let config = self.config_of(&located.key);
        let site = block::Site {
            view: self.view,
            scopes: &self.scopes,
            key: &located.key,
            id: &located.path,
            kind: &located.kind,
            ordinal: block::ordinal(
                self.view,
                &self.scopes,
                &text,
                (&located.key, &located.path, &located.kind),
            ),
            config,
        };
        let new = change(&site, &text)?;
        if new != text {
            self.texts.insert(located.key, new);
        }
        Ok(())
    }

    /// `change` applied to the members of `id`'s block: its fields and its verify statements. A union has none:
    /// refused first (`block::refuse_union`), as the text being planned parses, before the verb reads its field
    /// argument, so every member verb answers a union the same way (ADR 0219).
    fn on_members(
        &mut self,
        id: &str,
        change: impl FnOnce(&block::Site<'_>, &str) -> Result<String, Refusal<EditField>>,
    ) -> Result<(), Refusal<EditField>> {
        self.on(id, |site, text| {
            block::refuse_union(site, text)?;
            change(site, text)
        })
    }

    /// The path of the entity `edit` is about: the one a create makes, else the one its `entity_id` names.
    fn entity_path(&self, edit: &Edit) -> Result<String, Refusal<EditField>> {
        match edit {
            Edit::Create {
                kind, id, fields, ..
            } => Ok(self.new_path(kind, id, fields)),
            Edit::Set { entity_id, .. }
            | Edit::Unset { entity_id, .. }
            | Edit::Retitle { entity_id, .. }
            | Edit::Link { entity_id, .. }
            | Edit::Unlink { entity_id, .. }
            | Edit::AddVerify { entity_id, .. }
            | Edit::RemoveVerify { entity_id, .. }
            | Edit::Delete { entity_id, .. } => self.path_of(entity_id),
        }
    }

    /// What a set or unset of an entity's scope field moves (ADR 0284): the entity and every one scoped under it, as
    /// (old path, new path).
    fn moves(&self, edit: &Edit, path: &str) -> Vec<(String, String)> {
        let (Edit::Set { field, .. } | Edit::Unset { field, .. }) = edit else {
            return Vec::new();
        };
        let scopes = &self.scopes;
        let Some(kind) = scopes.kind(path) else {
            return Vec::new();
        };
        let scoped_by = &self.view.registries().graph_inputs().scopes;
        if scoped_by.get(kind.as_str()) != Some(field) {
            return Vec::new();
        }
        let scope = match edit {
            Edit::Set {
                value: Input::One(written),
                ..
            } => match scopes.resolve(None, written, &|_| true) {
                Found::One(found) => Some(found.to_string()),
                _ => Some(written.clone()),
            },
            _ => None,
        };
        let name = scopes.name(Sym::new(path));
        let new = match scope {
            Some(scope) => format!("{scope}/{name}"),
            None => name.to_string(),
        };
        let under = format!("{path}/");
        let mut moves = vec![(path.to_string(), new.clone())];
        for member in scopes.paths() {
            if let Some(rest) = member.as_str().strip_prefix(&under) {
                moves.push((member.to_string(), format!("{new}/{rest}")));
            }
        }
        moves
    }

    fn apply(&mut self, edit: &Edit) -> Result<(), Refusal<EditField>> {
        let path = self.entity_path(edit)?;
        let moves = self.moves(edit, &path);
        self.declared.edit(edit, &path);
        for (old, new) in &moves {
            self.declared.moved(old, new);
        }
        let creates = matches!(edit, Edit::Create { .. });
        let before = (creates || !moves.is_empty()).then(|| self.snapshot());
        self.apply_edit(edit, &path)?;
        let Some(before) = before else {
            return Ok(());
        };
        // A move or a create keeps what every other token names (ADR 0284).
        let (after, written, created) = match edit {
            Edit::Create { kind, .. } => (
                self.scopes
                    .added(Sym::new(&path), Sym::new(kind), keep::parent(&path)),
                None,
                Some(path.as_str()),
            ),
            Edit::Set { field, .. } | Edit::Unset { field, .. } => (
                keep::rekeyed(&self.scopes, &moves),
                Some((path.as_str(), field.as_str())),
                None,
            ),
            _ => return Ok(()),
        };
        let rekey = keep::Rekey {
            after,
            moves: &moves,
            written,
            created,
        };
        self.keep_targets(&before, &rekey)?;
        // The edits after this one read the project as it is now.
        let files: Vec<_> = moves
            .iter()
            .map(|(old, new)| (new.clone(), self.file_of(old)))
            .collect();
        for (old, _) in &moves {
            self.files.insert(old.clone(), None);
        }
        self.files.extend(files);
        self.scopes = rekey.after;
        Ok(())
    }

    fn apply_edit(&mut self, edit: &Edit, path: &str) -> Result<(), Refusal<EditField>> {
        let path = path.to_string();
        match edit {
            Edit::Set {
                field,
                value,
                annotations,
                ..
            } => self.on_members(&path, |site, text| {
                block::set(site, text, field, value, annotations.as_deref())
            }),
            Edit::Unset { field, .. } => {
                self.on_members(&path, |site, text| block::unset(site, text, field))
            }
            Edit::Retitle { title, .. } => {
                create::require_title(title)?;
                self.on(&path, |site, text| block::retitle(site, text, title))
            }
            Edit::Link { field, target, .. } => self.on_members(&path, |site, text| {
                block::link(site, text, field, target, true)
            }),
            Edit::Unlink { field, target, .. } => self.on_members(&path, |site, text| {
                block::link(site, text, field, target, false)
            }),
            Edit::AddVerify {
                kind,
                text: description,
                ..
            } => self.on_members(&path, |site, text| {
                block::verify(site, text, kind.as_deref(), description, true)
            }),
            Edit::RemoveVerify {
                kind,
                text: description,
                ..
            } => self.on_members(&path, |site, text| {
                block::verify(site, text, kind.as_deref(), description, false)
            }),
            Edit::Create {
                kind,
                id,
                title,
                file,
                fields,
                verify,
            } => self.create(kind, id, title, file, fields, verify),
            Edit::Delete {
                unlink_references, ..
            } => self.delete(&path, *unlink_references),
        }
    }

    fn finish(self) -> Result<EditPlan, Refusal<EditField>> {
        let project = self.view.project();
        let change = PlannedChange::new(
            self.view.env().sources().clone(),
            "cannot apply the edits",
            self.texts.into_iter().map(|(key, after)| {
                let before = project.source_text(&key);
                let line_break = LineBreak::of(before.as_deref().unwrap_or(""));
                (key, before, line_break.write(&after).into_owned())
            }),
        );
        let added = if change.is_empty() {
            Vec::new()
        } else {
            validate::check(self.view, &change.after(), &self.declared)?
        };
        let report = EditReport {
            entities: change.entities(),
            affected_files: change.affected_files().map(String::from).collect(),
            diff: change.diff(),
            added: DiagnosticList(added),
        };
        Ok(EditPlan { change, report })
    }
}

/// What an edit is, for the refusal that names it.
fn describe(edit: &Edit) -> String {
    match edit {
        Edit::Create { kind, id, .. } => format!("create {kind} {id}"),
        Edit::Set {
            entity_id, field, ..
        } => format!("set {entity_id} {field}"),
        Edit::Unset { entity_id, field } => format!("unset {entity_id} {field}"),
        Edit::Retitle { entity_id, .. } => format!("retitle {entity_id}"),
        Edit::Link {
            entity_id, field, ..
        } => format!("link {entity_id} {field}"),
        Edit::Unlink {
            entity_id, field, ..
        } => format!("unlink {entity_id} {field}"),
        Edit::AddVerify { entity_id, .. } => format!("add_verify {entity_id}"),
        Edit::RemoveVerify { entity_id, .. } => format!("remove_verify {entity_id}"),
        Edit::Delete { entity_id, .. } => format!("delete {entity_id}"),
    }
}

/// Plan `edits`, in order, over the project `view` reads: each edit reads the text the ones before it left.
/// Refused with the first edit that cannot be made (with several edits its message starts with
/// `edit <n> (<op> <entity>): `). Each file is laid out with its format configuration, read once per plan
/// through the view's disk (ADR 0169).
pub fn plan(view: &ProjectView<'_>, edits: &[Edit]) -> Result<EditPlan, Refusal<EditField>> {
    crate::config::refuse_rewrite(view.env().config_problems())?;
    view.project_root()?;
    let disk = view.containment().disk();
    let mut planner = Planner {
        view,
        configs: Configs::new(Forms::none(), &disk),
        texts: BTreeMap::new(),
        scopes: view.graph().scopes().clone(),
        files: BTreeMap::new(),
        declared: declared::Declared::default(),
    };
    for (n, edit) in edits.iter().enumerate() {
        planner.apply(edit).map_err(|mut refusal| {
            if edits.len() > 1 {
                refusal.error =
                    refusal
                        .error
                        .prefixed(format!("edit {} ({}): ", n + 1, describe(edit)));
            }
            refusal
        })?;
    }
    planner.finish()
}
