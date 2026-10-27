//! Which tokens of a file name an entity, read the way the graph build reads them (ADR 0163): the one finder rename,
//! the editor's find-references and a delete's referrer scan ask, over the compiled parses or the texts a plan holds.

use crate::build::is_define_block;
use crate::declare::declared_path;
use crate::defaults::is_defaults_block;
use crate::derive::{DerivedName, derived_names};
use crate::scope::{Found, Scopes};
use crate::{FieldForms, FileDefaults, GraphInputs, Reading};
use specforge_common::{SourceSpan, Sym};
use specforge_parser::{FieldEntry, MethodDecl, SpecFile};
use std::borrow::Cow;
use std::fmt;

/// Who writes a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holder {
    /// A block, by its id.
    Block(Sym),
    /// The file's `defaults` block for a kind (ADR 0149): no node, and no typed edit writes it (ADR 0151 D5).
    Defaults(Sym),
    /// A selective `use` statement's binding (ADR 0233): no node, no block holds it.
    Use,
}

impl fmt::Display for Holder {
    /// A block's id, or `defaults <kind>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(id) => write!(f, "{id}"),
            Self::Defaults(kind) => write!(f, "defaults {kind}"),
            Self::Use => f.write_str("use"),
        }
    }
}

/// How a token is written: what a typed edit can do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// The value of a declared `reference` field: `unset` removes it.
    Single,
    /// An item of a declared list field, or a list written as one value (ADR 0156 D1): `unlink` removes it.
    Item,
    /// A bare word or scheme ref ID in a field its kind does not declare (ADR 0093 D5).
    Undeclared,
    /// A type name a derived field links through (ADR 0127 D3): in a field written as a type, or a method signature.
    Derived,
    /// A `use` binding's imported name (never its alias): a rename edits it, a delete with `unlink_references`
    /// removes it (ADR 0233).
    Binding,
    /// A name before the last of a path (`v1.0` and `03` in `v1.0/03/D-01`, ADR 0284): it names the scope the path
    /// goes through. No block holds it, no edge is made of it, and a rename of that scope edits it.
    Qualifier,
}

/// One token of a file that names an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenToken {
    /// The path of the entity it names, as it resolved; as written when it resolves to none (E003, or a scope's
    /// short form that is ambiguous).
    pub target: Sym,
    pub holder: Holder,
    /// The blocks whose node holds it as a reference, sorted, once each: its block; each block that takes the default's
    /// entry (`Holder::Defaults`; whatever it writes in a derived field), but not one a derived name of the default
    /// names (a block never derives an edge to itself); none for a default no block takes or a `use` binding. The graph build links exactly these (the debug assertion of `GraphBuild`).
    pub held_by: Vec<Sym>,
    /// The entry's key, the derived field a type name links through, or, for a binding, the path its statement
    /// imports from.
    pub field: Sym,
    /// Where it writes the name: the name's own bytes, inside the quotes of a quoted reference (ADR 0218). For a
    /// path, the one segment this token names: the last for the reference, each earlier one for its qualifier.
    pub span: SourceSpan,
    /// Where the whole reference is written (a path in full, a rooted path with its `/`): `span` for a name that
    /// is no path.
    pub written: SourceSpan,
    pub form: Form,
}

impl WrittenToken {
    /// Where it is written, as a refusal names it: `file:line (holder, field)`.
    #[must_use]
    pub fn place(&self) -> String {
        format!(
            "{}:{} ({}, {})",
            self.span.file, self.span.start_line, self.holder, self.field
        )
    }
}

/// Every token of `file` that names an entity, in written order, read the way the graph build reads it:
/// - each applied entry of a `defaults` block, whether or not a block takes it (an E077 or E078 entry is not applied
///   and names nothing, ADR 0149 D4/D5), as [`Holder::Defaults`];
/// - each block's own entries, as the build stores them (coerced once, ADR 0163): their references
///   ([`Reading::references`], ADR 0093) and the type names a derived field of the holder's kind reads in them, that
///   name an entity its target admits, `scopes` answering what a written name or path names from the block (its
///   scope chain; its scope field and a binding read from the project, ADR 0284); never the block's own id;
/// - each selective `use` binding's imported name, held by none, as [`Holder::Use`] (ADR 0233); its alias names
///   nothing;
/// - each names the blocks that hold it (`held_by`).
///
/// A reference that is a path is a token for its last name, and a [`Form::Qualifier`] token for each name before
/// it that names a scope. A `define` block (never a node, W143) is skipped. A reference without a span (a value built
/// without a position) is not a token. Unlike the linker, a declared reference is a token whatever it resolves to
/// (E003, E022): it is written there. A duplicate declaration (E002, W060) is a block like any other: one file
/// cannot tell which wins.
pub fn written_tokens(file: &SpecFile, inputs: &GraphInputs, scopes: &Scopes) -> Vec<WrittenToken> {
    let forms = &inputs.field_forms;
    let (defaults, _) = FileDefaults::of(file, forms);
    let read = Reader { inputs, scopes };
    let mut tokens = Vec::new();
    for (kind, entry) in defaults.applied() {
        let holder = Holder::Defaults(kind);
        let mut takers: Vec<Sym> = defaults
            .takers(file, kind, entry.key.as_str())
            .map(|block| declared_path(inputs, &defaults, scopes, block).1)
            .collect();
        takers.sort_unstable();
        takers.dedup();
        let from = tokens.len();
        let entries = stored(forms, kind.as_str(), std::slice::from_ref(entry));
        // Each taker reads a default from its own scope chain; one nobody takes, from the project's.
        let readers: Vec<Option<Sym>> = if takers.is_empty() {
            vec![None]
        } else {
            takers.iter().map(|t| Some(*t)).collect()
        };
        for reader in readers {
            let mut mine = Vec::new();
            for entry in entries.iter() {
                read.entry(kind.as_str(), entry, holder, reader, &mut mine);
            }
            read.derived(kind.as_str(), &entries, &[], holder, reader, &mut mine);
            for token in &mut mine {
                if token.form != Form::Qualifier {
                    token.held_by = reader.into_iter().collect();
                }
            }
            tokens.extend(mine);
        }
        let merged = merged(tokens.split_off(from));
        tokens.extend(merged);
    }
    for entity in &file.entities {
        if is_defaults_block(entity) || is_define_block(entity) {
            continue;
        }
        let kind = entity.kind.raw.as_str();
        let id = declared_path(inputs, &defaults, scopes, entity).1;
        let holder = Holder::Block(id);
        let from = tokens.len();
        let entries = stored(forms, kind, entity.fields.entries());
        for entry in entries.iter() {
            read.entry(kind, entry, holder, Some(id), &mut tokens);
        }
        read.derived(
            kind,
            &entries,
            &entity.methods,
            holder,
            Some(id),
            &mut tokens,
        );
        for token in &mut tokens[from..] {
            if token.form != Form::Qualifier {
                token.held_by = vec![id];
            }
        }
    }
    // A `use` binding's imported name names its entity wherever it is declared; its alias names nothing (ADR 0233).
    for import in &file.imports {
        for binding in import
            .bindings
            .iter()
            .flatten()
            .filter(|b| !b.name.is_empty())
        {
            let found = scopes.resolve(None, &binding.name, &|_| true);
            read.push(
                &binding.name,
                &binding.name_span,
                resolved(&found),
                Holder::Use,
                import.path,
                Form::Binding,
                &mut tokens,
            );
        }
    }
    let at = |t: &WrittenToken| {
        (
            t.span.start_line,
            t.span.start_col,
            t.span.end_line,
            t.span.end_col,
        )
    };
    tokens.sort_by_key(at);
    // A name the build reads twice at one span (a default's derived name) is one token.
    tokens.dedup();
    tokens
}

/// The path a resolution links, if it does (an entity of the wrong kind is linked, and reported, all the same).
fn resolved(found: &Found) -> Option<Sym> {
    match found {
        Found::One(path) | Found::Mistyped(path) => Some(*path),
        Found::Ambiguous(_) | Found::None(_) => None,
    }
}

/// `tokens` with those that differ only in who holds them made one, holding for all of them.
fn merged(tokens: Vec<WrittenToken>) -> Vec<WrittenToken> {
    let mut merged: Vec<WrittenToken> = Vec::new();
    for token in tokens {
        let same = |m: &&mut WrittenToken| {
            WrittenToken {
                held_by: Vec::new(),
                ..(*m).clone()
            } == WrittenToken {
                held_by: Vec::new(),
                ..token.clone()
            }
        };
        match merged.iter_mut().find(same) {
            Some(into) => {
                into.held_by.extend(token.held_by);
                into.held_by.sort_unstable();
                into.held_by.dedup();
            }
            None => merged.push(token),
        }
    }
    merged
}

/// `entries` as the graph build stores them (`FieldForms::stored`): borrowed when no value is coerced.
fn stored<'e>(forms: &FieldForms, kind: &str, entries: &'e [FieldEntry]) -> Cow<'e, [FieldEntry]> {
    let mut owned: Option<Vec<FieldEntry>> = None;
    for (i, entry) in entries.iter().enumerate() {
        if let Cow::Owned(value) = forms.stored(kind, entry) {
            owned.get_or_insert_with(|| entries.to_vec())[i].value = value;
        }
    }
    owned.map_or(Cow::Borrowed(entries), Cow::Owned)
}

/// What a token needs from the graph: its inputs and who is in which scope.
struct Reader<'a> {
    inputs: &'a GraphInputs,
    scopes: &'a Scopes,
}

impl Reader<'_> {
    /// The tokens `written` makes, written at `span`: for the entity it resolved to (`target`), one for its last
    /// name and a [`Form::Qualifier`] for each name before it; else one for the text, which names nothing.
    #[allow(clippy::too_many_arguments, reason = "one token's every part")]
    fn push(
        &self,
        written: &str,
        span: &SourceSpan,
        target: Option<Sym>,
        holder: Holder,
        field: Sym,
        form: Form,
        out: &mut Vec<WrittenToken>,
    ) {
        let token = |target: Sym, at: SourceSpan, form: Form| WrittenToken {
            target,
            holder,
            held_by: Vec::new(),
            field,
            span: at,
            written: span.clone(),
            form,
        };
        let Some(target) = target else {
            out.push(token(Sym::new(written), span.clone(), form));
            return;
        };
        if !written.contains('/') {
            out.push(token(target, span.clone(), form));
            return;
        }
        // A scheme ref id (`gh.pr:org/repo`) holds a `/` and is one name.
        let rooted = usize::from(written.starts_with('/'));
        let names: Vec<&str> = if written.contains(':') {
            vec![written]
        } else {
            written[rooted..].split('/').collect()
        };
        // The path of the entity each name before the last names: the target's path, a segment shorter each.
        let mut prefixes: Vec<&str> = Vec::new();
        let mut path = target.as_str();
        for _ in 1..names.len() {
            let Some(cut) = path.rfind('/') else { break };
            path = &path[..cut];
            prefixes.push(path);
        }
        prefixes.reverse();
        let mut offset = rooted;
        for (i, name) in names.iter().enumerate() {
            let mut at = span.clone();
            at.start_col = span.start_col + offset;
            at.end_col = at.start_col + name.len();
            at.end_line = at.start_line;
            offset += name.len() + 1;
            if i + 1 == names.len() {
                out.push(token(target, at, form));
            } else if let Some(prefix) = prefixes.get(i) {
                out.push(token(Sym::new(prefix), at, Form::Qualifier));
            }
        }
    }

    /// The tokens of the references `entry` writes in a block of `kind`; `entry` is already as the graph build
    /// stores it (see [`stored`]). `from` is the block reading it, whose scope chain answers; a scope field reads
    /// from the project, with the unique suffix, as the linker does (ADR 0284).
    fn entry(
        &self,
        kind: &str,
        entry: &FieldEntry,
        holder: Holder,
        from: Option<Sym>,
        out: &mut Vec<WrittenToken>,
    ) {
        let forms = &self.inputs.field_forms;
        // A reference inside a typed block is the entity's, under its declared member path (ADR 0285).
        for (path, reading, entry) in forms.readings(kind, entry) {
            let field = path.declared();
            let form = match reading {
                _ if reading.is_single_reference() => Form::Single,
                Reading::Undeclared => Form::Undeclared,
                _ => Form::Item,
            };
            let target = forms.target(kind, &field);
            let admits = |k: &str| target.as_ref().is_none_or(|t| t.admits(k));
            let scope_field = self
                .inputs
                .scopes
                .get(kind)
                .is_some_and(|scope| *scope == field);
            for reference in reading.references(entry) {
                let Some(span) = reference.span else { continue };
                let found = if scope_field {
                    self.scopes.resolve(None, reference.id, &admits)
                } else {
                    self.scopes.chain(from, reference.id, &admits)
                };
                self.push(
                    reference.id,
                    &span,
                    resolved(&found),
                    holder,
                    Sym::new(&field),
                    form,
                    out,
                );
            }
        }
    }

    /// The type names a derived field of `kind` reads in `entries` and `methods`, that name an entity its target
    /// admits from `from`'s scope chain.
    fn derived(
        &self,
        kind: &str,
        entries: &[FieldEntry],
        methods: &[MethodDecl],
        holder: Holder,
        from: Option<Sym>,
        out: &mut Vec<WrittenToken>,
    ) {
        let forms = &self.inputs.field_forms;
        for reference in &self.inputs.derived_references {
            if reference.source_kind != kind {
                continue;
            }
            for DerivedName { written, path, at } in
                derived_names(reference, forms, entries, methods, from, self.scopes)
            {
                let Some(at) = at else { continue };
                self.push(
                    written,
                    &at,
                    Some(path),
                    holder,
                    Sym::new(&reference.field),
                    Form::Derived,
                    out,
                );
            }
        }
    }
}
