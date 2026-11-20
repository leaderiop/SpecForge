//! The entity a person's spelling names in a command's graph, read the way the host's surfaces read it
//! (`specforge query <entity>`, a `set_verdict`'s entity; ADR 0284, ADR 0315), so that a command checks the very entity
//! the host then writes to: the exact path; else, for a spelling not rooted with `/`, the one path that ends with it on
//! a segment boundary; else the numeric reading, where a numeric segment matches whatever its zero padding (`v1.0/1`
//! names `v1.0/01`). A rooted spelling (`/v1.0/01`) names a whole path, never a suffix. Several answers at one step are
//! [`Found::Ambiguous`], never a pick.

use specforge_extension_sdk::prelude::*;

/// What a spelling names.
#[derive(Debug, PartialEq)]
pub enum Found<'g> {
    One(&'g GraphNode),
    /// The paths it names alike, sorted.
    Ambiguous(Vec<&'g str>),
    None,
}

impl<'g> Found<'g> {
    /// The one entity, or the refusal naming why there is none: `'<spelling>' names no entity` or `'<spelling>' is
    /// ambiguous: <paths>`.
    pub fn one(self, spelling: &str) -> Result<&'g GraphNode, String> {
        match self {
            Found::One(node) => Ok(node),
            Found::Ambiguous(paths) => Err(format!(
                "'{spelling}' is ambiguous: {}; name one by its path",
                paths.join(", ")
            )),
            Found::None => Err(format!("'{spelling}' names no entity of the project")),
        }
    }
}

/// [`Found`] of `many`: one, several or none.
fn of<'g>(mut many: Vec<&'g GraphNode>) -> Found<'g> {
    many.sort_by(|a, b| a.id.cmp(&b.id));
    match many.as_slice() {
        [] => Found::None,
        [one] => Found::One(one),
        _ => Found::Ambiguous(many.iter().map(|n| n.id.as_str()).collect()),
    }
}

/// The entity `spelling` names in `graph`, as the module says.
pub fn find<'g>(graph: &'g CommandGraph, spelling: &str) -> Found<'g> {
    let (rooted, path) = match spelling.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, spelling),
    };
    // A scheme ref id (`file:src/a.js`) is one name, looked up whole.
    let segmented = !path.contains(':');
    if path.is_empty() || segmented && path.split('/').any(str::is_empty) {
        return Found::None;
    }
    if let Some(node) = graph.node(path) {
        return Found::One(node);
    }
    if !rooted && segmented {
        let tail = format!("/{path}");
        match of(graph
            .nodes()
            .iter()
            .filter(|n| n.id.ends_with(&tail))
            .collect())
        {
            Found::None => {}
            found => return found,
        }
    }
    if segmented {
        numeric(graph, rooted, path)
    } else {
        Found::None
    }
}

/// The numeric reading of `path`: the entities whose last segments, as many as `path` has, each equal its segment or
/// are numeric ids that normalize alike (every segment, for a `rooted` spelling). `None` when `path` has no numeric
/// segment: the exact and suffix steps already said all there is.
fn numeric<'g>(graph: &'g CommandGraph, rooted: bool, path: &str) -> Found<'g> {
    let wanted: Vec<&str> = path.split('/').collect();
    if !wanted.iter().any(|s| numeric_key(s).is_some()) {
        return Found::None;
    }
    let alike = |segment: &str, written: &str| {
        segment == written
            || numeric_key(segment).is_some_and(|key| numeric_key(written).as_ref() == Some(&key))
    };
    of(graph
        .nodes()
        .iter()
        .filter(|n| {
            let segments: Vec<&str> = n.id.split('/').collect();
            (segments.len() == wanted.len() || !rooted && segments.len() > wanted.len())
                && segments[segments.len() - wanted.len()..]
                    .iter()
                    .zip(&wanted)
                    .all(|(segment, written)| alike(segment, written))
        })
        .collect())
}

/// A numeric id segment normalized: its digits without leading zeros, its one trailing letter and its dotted decimals
/// (`01`, `1`, `2A`, `02.1`, `1.2.3`); `None` for any other segment (`01-01`, `TR1`, `v1.0`).
fn numeric_key(segment: &str) -> Option<(String, Option<char>, String)> {
    let (head, decimals) = segment.split_once('.').unwrap_or((segment, ""));
    let digits = head.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let letter = head[digits.len()..]
        .chars()
        .next()
        .filter(|_| head.len() == digits.len() + 1);
    let well_formed = !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && (head.len() == digits.len() || letter.is_some())
        && (decimals.is_empty() && !segment.contains('.')
            || decimals
                .split('.')
                .all(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit())));
    well_formed.then(|| {
        let whole = digits.trim_start_matches('0');
        let whole = if whole.is_empty() { "0" } else { whole };
        (whole.to_string(), letter, decimals.to_string())
    })
}

#[cfg(test)]
mod tests;
