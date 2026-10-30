//! Re-pointing (D-r42-5): upstream's text reads its references through `~/.claude/gsd-core/<path>`, a directory this
//! port does not install. At render time every such path becomes the command that serves the embedded file, so the
//! assets stay byte-equal to upstream's.

use super::assets;

/// The directory name every upstream install path ends in, after its prefix.
const MARKER: &str = ".claude/gsd-core/";
/// What can stand before `.claude/`, longest first.
const PREFIXES: [&str; 4] = ["${HOME}/", "$HOME/", "~/", "./"];
/// Said after a path that no ported workflow embeds: upstream has it, `@gsd/core` does not.
const LATER: &str = " (not part of @gsd/core)";

/// A file the user's own profile run writes there: not an asset, so its path stays what it says.
const USER_FILES: [&str; 1] = ["USER-PROFILE.md"];

fn is_path(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-')
}

/// `text` with each `[@]<prefix>.claude/gsd-core/<path>` replaced by `` `specforge gsd reference gsd-core/<path>` ``.
pub fn repoint(text: &str) -> String {
    repoint_for(text, "claude-code")
}

/// [`repoint`] for the text of `host`: the calls it writes carry `--host` unless the host is Claude Code (the default),
/// so a file read through `reference` is converted for the host that asked.
pub fn repoint_for(text: &str, host: &str) -> String {
    let flag = if host == "claude-code" {
        String::new()
    } else {
        format!(" --host {host}")
    };
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
        if USER_FILES.contains(&path) {
            out.push_str(&rest[..end]);
            rest = &rest[end..];
            continue;
        }
        let full = format!("gsd-core/{path}");
        // Inside a double-quoted shell string a backtick runs a command: say the command bare there.
        let quoted = rest[..start]
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .matches('"')
            .count()
            % 2
            == 1;
        let wrapped = rest[..start].ends_with('`') && rest[end..].starts_with('`');
        out.push_str(&rest[..start - usize::from(wrapped)]);
        let command = format!("specforge gsd reference {full}{flag}");
        let tick = if quoted { "" } else { "`" };
        out.push_str(&format!("{tick}{command}{tick}"));
        if !quoted && assets::get(&full).is_none() {
            out.push_str(LATER);
        }
        rest = &rest[end + usize::from(wrapped)..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests;
