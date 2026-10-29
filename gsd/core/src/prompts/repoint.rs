//! Re-pointing (D-r42-5): upstream's text reads its references through `~/.claude/gsd-core/<path>`, a directory this
//! port does not install. At render time every such path becomes the command that serves the embedded file, so the
//! assets stay byte-equal to upstream's.

use super::assets;

/// The directory name every upstream install path ends in, after its prefix.
const MARKER: &str = ".claude/gsd-core/";
/// What can stand before `.claude/`, longest first.
const PREFIXES: [&str; 4] = ["${HOME}/", "$HOME/", "~/", "./"];
/// Said after a path that no ported workflow embeds yet.
const LATER: &str = " (not installed by @gsd/core yet)";

fn is_path(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-')
}

/// `text` with each `[@]<prefix>.claude/gsd-core/<path>` replaced by `` `specforge gsd reference gsd-core/<path>` ``.
pub fn repoint(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(MARKER) {
        let before = &rest[..at];
        let Some(prefix) = PREFIXES.iter().find(|p| before.ends_with(**p)) else {
            // Another spelling (a shell variable): upstream's line is a ported one and was rewritten.
            out.push_str(&rest[..at + MARKER.len()]);
            rest = &rest[at + MARKER.len()..];
            continue;
        };
        let mut start = at - prefix.len();
        if rest[..start].ends_with('@') {
            start -= 1;
        }
        let tail = &rest[at + MARKER.len()..];
        let length = tail.find(|c: char| !is_path(c)).unwrap_or(tail.len());
        let path = tail[..length].trim_end_matches(['.', ',', ';', ':']);
        let end = at + MARKER.len() + path.len();
        let full = format!("gsd-core/{path}");
        let wrapped = rest[..start].ends_with('`') && rest[end..].starts_with('`');
        out.push_str(&rest[..start - usize::from(wrapped)]);
        let command = format!("specforge gsd reference {full}");
        out.push('`');
        out.push_str(&command);
        out.push('`');
        if assets::get(&full).is_none() {
            out.push_str(LATER);
        }
        rest = &rest[end + usize::from(wrapped)..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests;
