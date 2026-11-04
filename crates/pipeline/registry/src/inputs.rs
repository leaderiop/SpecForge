//! A call's inputs (ADR 0337 D2): the one table of what each call kind is served, and the problems of the inputs an
//! extension declares, which the declaration check reports (W021) and the host serves empty, keeping the call. A file
//! read's own problems are ADR 0293's.

use std::collections::HashSet;

use specforge_protocol_types::{ExtensionDeclaration, FileRead, Input, InputKind, Root};

use crate::compilation::populate::keyword;

/// The kinds of call that declare inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    Pass,
    Command,
    Prompt,
    Hook,
    Collector,
}

/// Whether a call of `kind` is served `input`: exactly what each kind was served before the fold. A pass gets the
/// entities and a command and a writing collector the graph without asking; a hook's files are its tool's paths
/// (ADR 0293 D6).
pub fn served(kind: CallKind, input: &Input) -> bool {
    use CallKind::*;
    match input.kind() {
        InputKind::Files => matches!(kind, Pass | Command | Collector),
        InputKind::Tests => kind == Pass,
        InputKind::State => matches!(kind, Command | Prompt | Hook),
        InputKind::Graph => matches!(kind, Prompt | Hook),
        InputKind::SlashCommands => kind == Prompt,
        InputKind::Git | InputKind::Setting => kind == Hook,
    }
}

/// Why `read` cannot be served as declared, or `None` when its shape is sound: a name that is no word, neither or both
/// of `field` and `glob`, a field that is not `kind.field`, a glob that is absolute, climbs with `..`, has an empty
/// segment or more than [`MAX_DOUBLE_STAR`] `**` segments, `root` on a glob, or a `root` that is no root's name. A name repeated in an owner's reads is
/// [`problems`]'.
pub fn read_problem(read: &FileRead) -> Option<String> {
    if !is_word(&read.name) {
        return Some(format!(
            "its name '{}' is not a word [a-z][a-z0-9_]*",
            read.name
        ));
    }
    match (&read.field, &read.glob) {
        (None, None) => Some("it names neither a `field` nor a `glob`".to_string()),
        (Some(_), Some(_)) => Some("it names both a `field` and a `glob`".to_string()),
        (Some(field), None) => {
            if field.split('.').count() < 2 || field.split('.').any(str::is_empty) {
                return Some(format!("its field '{field}' is not `kind.field`"));
            }
            match read.root.as_deref() {
                Some(other) if Root::parse(other).is_none() => {
                    Some(format!("its root '{other}' is no root's name"))
                }
                _ => None,
            }
        }
        (None, Some(glob)) => {
            if read.root.is_some() {
                return Some(
                    "a glob is always from the project root: it takes no `root`".to_string(),
                );
            }
            glob_problem(glob)
        }
    }
}

/// The most `**` segments a glob may have: the matcher tries every split of the path at each one.
pub const MAX_DOUBLE_STAR: usize = 4;

fn glob_problem(glob: &str) -> Option<String> {
    let absolute =
        glob.starts_with('/') || glob.starts_with('\\') || glob.as_bytes().get(1) == Some(&b':');
    if glob.is_empty() || absolute || glob.contains('\\') || glob.contains('\0') {
        return Some(format!(
            "its glob '{glob}' is empty, absolute or not `/` separated"
        ));
    }
    if glob
        .split('/')
        .any(|segment| segment == ".." || segment.is_empty())
    {
        return Some(format!(
            "its glob '{glob}' climbs with `..` or has an empty segment"
        ));
    }
    if glob.split('/').filter(|segment| *segment == "**").count() > MAX_DOUBLE_STAR {
        return Some(format!(
            "its glob '{glob}' has more than {MAX_DOUBLE_STAR} `**` segments"
        ));
    }
    None
}

fn is_word(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Why the `setting` input `extension:name` of `owner`'s call is served empty: it names `owner` itself (its own
/// settings are read with `Settings::current`), or not an extension and a setting.
pub fn setting_problem(owner: &str, extension: &str, name: &str) -> Option<&'static str> {
    if extension == owner {
        Some("it names its own extension: read it with Settings::current")
    } else if extension.is_empty() || name.is_empty() {
        Some("it does not name an extension and a setting")
    } else {
        None
    }
}

/// The messages (W021) about the inputs `declaration` makes: an input its call kind is not served, a second input of
/// a tag that is one per call (all but `files` and `setting`), a `setting` with a [`setting_problem`], a read with a
/// [`read_problem`], a name two reads of one owner share, and a field no loaded extension declares.
pub(crate) fn problems(
    declaration: &ExtensionDeclaration,
    loaded: &[ExtensionDeclaration],
) -> Vec<String> {
    let name = declaration.name();
    let mut messages = Vec::new();
    let mut check = |owner: String, kind: CallKind, inputs: &[Input]| {
        let mut tags = HashSet::new();
        let mut names = HashSet::new();
        for input in inputs {
            let tag = input.tag();
            let said = |what: String, why: &str| {
                format!("extension '{name}': {owner} declares {what}, which is not served: {why}")
            };
            if !served(kind, input) {
                messages.push(said(
                    format!("the input '{tag}'"),
                    "its kind of call is not served it",
                ));
                continue;
            }
            let read = match input {
                Input::Files(read) => read,
                Input::Setting {
                    extension,
                    name: setting,
                } => {
                    if let Some(why) = setting_problem(name, extension, setting) {
                        messages.push(said(format!("the input '{extension}:{setting}'"), why));
                    }
                    continue;
                }
                _ => {
                    if !tags.insert(tag) {
                        messages.push(said(format!("the input '{tag}'"), "it is declared twice"));
                    }
                    continue;
                }
            };
            let said = |why: &str| said(format!("the read '{}'", read.name), why);
            if let Some(why) = read_problem(read) {
                messages.push(said(&why));
            } else if !names.insert(read.name.as_str()) {
                messages.push(said("another read of the same owner has its name"));
            } else if let Some(field) = &read.field
                && !declares_field(loaded, field)
            {
                messages.push(said(&format!(
                    "no loaded extension declares the field '{field}'"
                )));
            }
        }
    };
    for pass in &declaration.passes {
        check(
            format!("pass '{}'", pass.name),
            CallKind::Pass,
            &pass.inputs,
        );
    }
    let surfaces = &declaration.surfaces;
    for command in &surfaces.commands {
        let owner = format!("command '{}'", command.id);
        check(owner, CallKind::Command, &command.inputs);
    }
    for prompt in &surfaces.prompts {
        check(
            format!("prompt '{}'", prompt.id),
            CallKind::Prompt,
            &prompt.inputs,
        );
    }
    for hook in &surfaces.hooks {
        check(format!("hook '{}'", hook.id), CallKind::Hook, &hook.inputs);
    }
    for collector in &declaration.collectors {
        let owner = format!("collector '{}'", collector.name);
        check(owner, CallKind::Collector, &collector.inputs);
    }
    messages
}

/// Whether a loaded extension declares `kind.field` or `kind.field.member` (a kind's own fields, the fields every kind
/// of its extension shares, and an enhancement's).
fn declares_field(loaded: &[ExtensionDeclaration], path: &str) -> bool {
    let Some((kind, declared)) = path.split_once('.') else {
        return false;
    };
    let found = |fields: &[specforge_protocol_types::FieldDescriptor]| {
        let mut hit = false;
        for field in fields {
            field.for_each_path(&field.name, &mut |at, _| hit |= at == declared);
        }
        hit
    };
    loaded.iter().any(|d| {
        d.entities
            .iter()
            .any(|k| keyword(k) == kind && (found(&k.fields) || found(&d.shared_fields)))
            || d.enhancements
                .iter()
                .any(|e| e.target_kind == kind && found(&e.fields))
    })
}
