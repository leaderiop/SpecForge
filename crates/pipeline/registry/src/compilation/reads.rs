//! The file reads a pass or command declares (ADR 0293): the problems of a read as declared, which the declaration
//! check reports (W021) and the host serves empty.

use std::collections::HashSet;

use specforge_protocol_types::{ExtensionDeclaration, FileRead};

use super::populate::keyword;

/// Why `read` cannot be served as declared, or `None` when its shape is sound: a name that is no word, neither or both
/// of `field` and `glob`, a field that is not `kind.field`, a glob that is absolute, climbs with `..`, has an empty
/// segment or more than [`MAX_DOUBLE_STAR`] `**` segments, `base` on a glob, or a `base` other than `spec` or `project`. A name repeated in an owner's reads is
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
            match read.base.as_deref() {
                None | Some("spec" | "project") => None,
                Some(other) => Some(format!(
                    "its base '{other}' is neither `spec` nor `project`"
                )),
            }
        }
        (None, Some(glob)) => {
            if read.base.is_some() {
                return Some(
                    "a glob is always from the project root: it takes no `base`".to_string(),
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

/// The messages (W021) about the reads `declaration` makes: a read with a [`read_problem`], a name two reads of one
/// owner share, and a field no loaded extension declares.
pub(crate) fn problems(
    declaration: &ExtensionDeclaration,
    loaded: &[ExtensionDeclaration],
) -> Vec<String> {
    let name = declaration.name();
    let mut messages = Vec::new();
    let mut check = |owner: String, reads: &[FileRead]| {
        let mut seen = HashSet::new();
        for read in reads {
            let said = |why: &str| {
                format!(
                    "extension '{name}': {owner} declares the read '{}', which is not served: {why}",
                    read.name
                )
            };
            if let Some(why) = read_problem(read) {
                messages.push(said(&why));
            } else if !seen.insert(read.name.as_str()) {
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
        check(format!("pass '{}'", pass.name), &pass.reads);
    }
    for command in &declaration.surfaces.commands {
        check(format!("command '{}'", command.id), &command.reads);
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
