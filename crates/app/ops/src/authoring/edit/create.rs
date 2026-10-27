//! Creating and deleting entities (ADR 0151): a new block goes after the last block of its kind in its file, laid
//! out canonically; a deleted block takes its doc comment and one blank line with it.

use super::block::{self, Block, Site};
use super::path::{self, FieldPath};
use super::write::not_editable;
use super::{EditField, Input, Obligation, Planner};
use crate::authoring::entity_id::validate_id;
use crate::authoring::rename;
use crate::read::navigate::{TextEdit, apply_edits};
use crate::{OpError, OpErrorKind, Refusal};
use specforge_common::Sym;
use specforge_graph::field_form::spell;
use specforge_graph::{Form, Holder, written_tokens};
use specforge_parser::parse;
use std::collections::BTreeSet;

/// A field of a block that names an entity: (entity, field, whether it is a list).
type Referrer = (String, String, bool);

/// The header line of a new `kind` block, its braces on the lines it opens (ADR 0175 D7).
pub(super) fn header(kind: &str, id: &str, title: &str) -> String {
    format!("{kind} {id} {} {{\n}}", spell::quoted(title))
}

/// `text` with `fields` set on `site`'s block and `verify` added, as a create writes them: one `set` per field, one
/// add-verify per obligation (ADR 0175 D7). A `field.entry` path is refused (`path::whole_field`).
pub(super) fn fill(
    site: &Site<'_>,
    mut text: String,
    fields: &std::collections::BTreeMap<String, Input>,
    verify: &[Obligation],
) -> Result<String, Refusal<EditField>> {
    for (name, input) in fields {
        if let FieldPath::Entry { field, entry } = path::resolve(site, name, None)? {
            return Err(path::whole_field("create", field, entry));
        }
        text = block::set(site, &text, name, input, None)?;
    }
    for o in verify {
        text = block::verify(site, &text, o.kind.as_deref(), &o.text, true)?;
    }
    Ok(text)
}

impl Planner<'_> {
    pub(super) fn create(
        &mut self,
        kind: &str,
        id: &str,
        title: &str,
        file: &str,
        fields: &std::collections::BTreeMap<String, Input>,
        verify: &[Obligation],
    ) -> Result<(), Refusal<EditField>> {
        validate_id(id, self.view.registries().kinds()).map_err(|e| e.about(EditField::Id))?;
        let graph = self.view.graph();
        if (graph.node(id).is_some() && !self.deleted.contains(id)) || self.created.contains_key(id)
        {
            return Err(OpError::new(
                OpErrorKind::Conflict,
                rename::TAKEN,
                format!("cannot create '{id}': an entity with that id exists"),
            )
            .with_entity(id)
            .about(EditField::Id));
        }
        self.view
            .kinds()
            .declared(kind)
            .map_err(|e| e.about(EditField::Kind))?;
        require_title(title)?;
        let file = self
            .view
            .source_place(file)
            .map_err(|e| e.about(EditField::File))?;
        let file = file.as_str();
        let existing = self.text(file).unwrap_or_default();
        let parsed = parse(&existing, file);
        let site = Site {
            view: self.view,
            key: file,
            id,
            kind,
            config: self.config_of(file),
        };
        let header = header(kind, id, title);
        let text = fill(
            &site,
            block::canonical(&site, &place(&existing, &parsed, kind, &header))?,
            fields,
            verify,
        )?;
        self.texts.insert(file.to_string(), text);
        self.created
            .insert(id.to_string(), (kind.to_string(), file.to_string()));
        self.deleted.remove(id);
        Ok(())
    }

    pub(super) fn delete(
        &mut self,
        id: &str,
        unlink_references: bool,
    ) -> Result<(), Refusal<EditField>> {
        let located = self.locate(id)?;
        if unlink_references {
            let (referrers, bound) = self.referrers(id, &located.kind)?;
            for (referrer, field, list) in referrers {
                self.declared.field(&referrer, &field);
                self.on(&referrer, |site, text| {
                    if list {
                        block::link(site, text, &field, id, false)
                    } else {
                        block::unset(site, text, &field)
                    }
                })?;
            }
            for key in bound {
                if let Some(text) = self.text(&key)
                    && let Some(new) = unbind(&text, &key, id)
                {
                    self.texts.insert(key, new);
                }
            }
        }
        self.on(id, |site, text| {
            let mut block = Block::open(site, text)?;
            block.delete();
            Ok(block.text())
        })?;
        self.deleted.insert(id.to_string());
        self.created.remove(id);
        Ok(())
    }

    /// Every field of another block that names `id` (of kind `kind`), as the graph's finder reads the texts the plan
    /// holds so far (ADR 0163): (entity, field, whether it is a list), in file order, and the keys of the files with a
    /// `use` binding that imports it (ADR 0233). A token no typed edit writes
    /// refuses the delete, naming each place: an entry of a `defaults` block (ADR 0151 D5), a field its holder's kind
    /// does not declare, or a type name a derived field reads.
    fn referrers(
        &self,
        id: &str,
        kind: &str,
    ) -> Result<(Vec<Referrer>, BTreeSet<String>), Refusal<EditField>> {
        let inputs = self.view.registries().graph_inputs();
        let graph = self.view.graph();
        // The entity being deleted is in the graph; one this plan created is not yet.
        let scopes = if graph.scopes().contains(id) {
            graph.scopes().clone()
        } else {
            graph.scopes().added(Sym::new(id), Sym::new(kind), None)
        };
        let mut keys: BTreeSet<String> = self.texts.keys().cloned().collect();
        keys.extend(
            self.view
                .project()
                .source_keys()
                .into_iter()
                .map(String::from),
        );
        let (mut found, mut elsewhere, mut bound) = (Vec::new(), Vec::new(), BTreeSet::new());
        for key in keys {
            let Some(text) = self.text(&key) else {
                continue;
            };
            for token in written_tokens(&parse(&text, &key), inputs, &scopes) {
                if token.target.as_str() != id {
                    continue;
                }
                match (token.holder, token.form) {
                    // The block being deleted takes its own tokens with it.
                    (Holder::Block(holder), _) if holder.as_str() == id => continue,
                    (Holder::Block(holder), Form::Single | Form::Item) => {
                        let referrer = (
                            holder.to_string(),
                            token.field.to_string(),
                            token.form == Form::Item,
                        );
                        if !found.contains(&referrer) {
                            found.push(referrer);
                        }
                        continue;
                    }
                    // A use binding names no field: the delete removes it (`unbind`).
                    (Holder::Use, _) => {
                        bound.insert(key.clone());
                        continue;
                    }
                    (Holder::Block(_) | Holder::Defaults(_), _) => {}
                }
                let at = token.place();
                if !elsewhere.contains(&at) {
                    elsewhere.push(at);
                }
            }
        }
        if elsewhere.is_empty() {
            return Ok((found, bound));
        }
        Err(not_editable(
            EditField::Entity,
            format!(
                "cannot unlink '{id}' where no typed edit writes: {}; remove it there first",
                elsewhere.join(", ")
            ),
        ))
    }
}

/// `text`, the planned text of source `key`, without the `use` bindings that import `id` (ADR 0233): a statement
/// keeps its other bindings, written `{ a, b as c }` as the formatter writes an import; one left with none goes
/// whole with its doc comment as the parse read it (never the format header) and one blank line beside it, as a
/// deleted block goes (`Block::delete`). `None` when no binding of `text`
/// imports `id`.
fn unbind(text: &str, key: &str, id: &str) -> Option<String> {
    let file = parse(text, key);
    let mut out = text.to_string();
    // Bottom-up, so the positions of the statements above stay those of `text`.
    for import in file.imports.iter().rev() {
        let Some(bindings) = &import.bindings else {
            continue;
        };
        let kept: Vec<String> = bindings
            .iter()
            .filter(|b| b.name != id)
            .map(|b| match &b.alias {
                Some(alias) => format!("{} as {alias}", b.name),
                None => b.name.clone(),
            })
            .collect();
        if kept.len() == bindings.len() {
            continue;
        }
        if let (Some(first), Some(last)) = (bindings.first(), bindings.last())
            && !kept.is_empty()
        {
            let mut span = first.span.clone();
            (span.end_line, span.end_col) = (last.span.end_line, last.span.end_col);
            let edit = TextEdit {
                span,
                new_text: kept.join(", "),
            };
            out = apply_edits(&out, [&edit])?;
            continue;
        }
        let mut lines: Vec<&str> = out.split('\n').collect();
        let first = import
            .doc_span
            .as_ref()
            .map_or(import.span.start_line, |doc| doc.start_line)
            - 1;
        let last = import.span.end_line - 1;
        block::cut(&mut lines, first, last);
        out = lines.join("\n");
    }
    (out != text).then_some(out)
}

/// `existing` with `block` after the last block of `kind`, else at the end, one blank line between.
fn place(existing: &str, parsed: &specforge_parser::SpecFile, kind: &str, block: &str) -> String {
    if existing.trim().is_empty() {
        return format!("{block}\n");
    }
    let mut lines: Vec<&str> = existing.split('\n').collect();
    let after = parsed
        .entities
        .iter()
        .filter(|e| e.kind.raw.as_str() == kind && e.span.end_line <= lines.len())
        .map(|e| e.span.end_line)
        .max();
    let new: Vec<&str> = std::iter::once("").chain(block.split('\n')).collect();
    match after {
        Some(end) => {
            lines.splice(end..end, new);
        }
        None => {
            if lines.last() == Some(&"") {
                lines.pop();
            }
            lines.extend(new);
            lines.push("");
        }
    }
    lines.join("\n")
}

/// A blank title, refused before anything is planned: the rule E076 asks (ADR 0225).
pub(super) fn require_title(title: &str) -> Result<(), Refusal<EditField>> {
    if specforge_graph::title::is_untitled(Some(title)) {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            "invalid_title",
            "a block needs a title",
        )
        .about(EditField::Title));
    }
    Ok(())
}
