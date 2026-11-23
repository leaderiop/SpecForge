//! Project-relative paths as the packs compute them from what a manifest, a source file or a report writes.

use crate::report::unescape;

/// `path` (relative, `/` or `\` separated) under the project-relative directory `dir`, its `.` and `..` resolved;
/// `None` when it leaves the project, or is absolute (`/…`, `\…`, a drive or a scheme: any `:`).
pub fn joined(dir: &str, path: &str) -> Option<String> {
    if path.starts_with(['/', '\\']) || path.contains(':') {
        return None;
    }
    let mut segments: Vec<&str> = Vec::new();
    for segment in dir.split('/').chain(path.split(['/', '\\'])) {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            segment => segments.push(segment),
        }
    }
    Some(segments.join("/"))
}

/// The path a report's URI names (a SARIF `artifactLocation.uri`), percent-decoded. A `file:` URI loses `file://`
/// and an empty or `localhost` host (`file:///p/x` and `file://localhost/p/x` give `/p/x`; another host gives
/// `//host/rest`, a UNC path), and `/C:/…` loses its leading `/`. Text with no scheme is a relative reference (as SARIF
/// writes with `uriBaseId`) and comes back decoded. Either way a `?` query or `#` fragment is dropped. A URI of
/// another scheme gives `None`. Only a path: [`relative`] decides whether it lies in the project, so a decoded `..`
/// cannot climb out.
pub fn file_uri_path(uri: &str) -> Option<String> {
    let uri = uri.split(['?', '#']).next().unwrap_or(uri);
    let path = match uri.split_once(':') {
        Some((scheme, rest)) if is_scheme(scheme) => {
            if !scheme.eq_ignore_ascii_case("file") {
                return None;
            }
            match rest.strip_prefix("//") {
                Some(authority) => {
                    let (host, path) =
                        authority.split_at(authority.find('/').unwrap_or(authority.len()));
                    if host.is_empty() || host.eq_ignore_ascii_case("localhost") {
                        path.to_string()
                    } else {
                        format!("//{host}{path}")
                    }
                }
                None => rest.to_string(),
            }
        }
        _ => uri.to_string(),
    };
    let path = unescape(&path);
    match path.strip_prefix('/') {
        Some(rest)
            if drive(rest).is_some() && matches!(rest.as_bytes().get(2), None | Some(b'/')) =>
        {
            Some(rest.to_string())
        }
        _ => Some(path),
    }
}

/// Whether `text` is a URI scheme (RFC 3986): a letter, then letters, digits, `+`, `-` or `.`; one letter alone is a
/// drive, not a scheme.
fn is_scheme(text: &str) -> bool {
    let mut chars = text.chars();
    text.len() > 1
        && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// The one warning line a pack returns for the paths a reader of `report` skipped as outside the project (ADR 0389 D7),
/// each already [`shown`](crate::shown): `None` when there is none, else the count and the first five, then how many
/// more. The host turns the line into a W203.
pub fn outside_line(report: &str, outside: &[String]) -> Option<String> {
    skipped_line(report, "file(s) outside the project", outside)
}

/// The one warning line a pack returns for the paths a reader of `report` skipped as naming no file ([`Place::NoFile`]),
/// each already [`shown`](crate::shown), worded as [`outside_line`]'s: `None` when there is none.
pub fn no_file_line(report: &str, no_file: &[String]) -> Option<String> {
    skipped_line(report, "path(s) naming no file", no_file)
}

/// `<report>: skipped <n> <what>: ` and the first five `paths`, then how many more; `None` for no path.
fn skipped_line(report: &str, what: &str, paths: &[String]) -> Option<String> {
    const NAMED: usize = 5;
    if paths.is_empty() {
        return None;
    }
    let mut line = format!(
        "{report}: skipped {} {what}: {}",
        paths.len(),
        paths[..paths.len().min(NAMED)].join(", ")
    );
    if paths.len() > NAMED {
        line.push_str(&format!(", and {} more", paths.len() - NAMED));
    }
    Some(line)
}

/// What a report's path names against the project root ([`place`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// A project file, relative to the root, with `/` separators.
    File(String),
    /// A path outside the project: absolute elsewhere, or climbing out through `..`.
    Outside,
    /// No file: a path that resolves to the project root itself (`.`, `a/..`, the root's own absolute path), or a
    /// compiler's pseudo file name in angle brackets (rustc's `<std macros>`, `<anon>`).
    NoFile,
}

/// What `path` names against the project root `cwd`. Both are read the same way first: a `\\?\` (or `\\?\UNC\`)
/// verbatim prefix is stripped, `\` is folded to `/`, a drive letter compares case-insensitively and a trailing `/` is
/// dropped. The host's `cwd` is the canonical root, so the other spelling of macOS's `/private` alias (`/private/var`
/// and `/var`, `/private/tmp` and `/tmp`) is tried too. A relative `path` is taken as project-relative already. Either
/// way its `.` and `..` segments are resolved: a path that climbs out of the project through `..` is
/// [`Place::Outside`], as an absolute one elsewhere is, and one that resolves to nothing is [`Place::NoFile`], as a
/// name in angle brackets is.
pub fn place(path: &str, cwd: &str) -> Place {
    if path.starts_with('<') && path.ends_with('>') {
        return Place::NoFile;
    }
    let path = normal(path);
    if !is_absolute(&path) {
        return contained(&path);
    }
    let cwd = normal(cwd);
    aliases(&cwd)
        .iter()
        .find_map(|root| {
            aliases(&path)
                .iter()
                .find_map(|spelling| under(spelling, root))
        })
        .map_or(Place::Outside, |rest| contained(&rest))
}

/// `path` relative to the project root `cwd` when it names a project file ([`place`]); `None` when it lies outside the
/// project or names no file.
pub fn relative(path: &str, cwd: &str) -> Option<String> {
    match place(path, cwd) {
        Place::File(file) => Some(file),
        Place::Outside | Place::NoFile => None,
    }
}

/// The project-relative `path` with its `.` and `..` segments resolved: [`Place::Outside`] when a `..` climbs out of
/// the project, [`Place::NoFile`] when nothing is left.
fn contained(path: &str) -> Place {
    let mut kept: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if kept.pop().is_none() {
                    return Place::Outside;
                }
            }
            name => kept.push(name),
        }
    }
    if kept.is_empty() {
        Place::NoFile
    } else {
        Place::File(kept.join("/"))
    }
}

/// `path` with no verbatim prefix, `/` separators and no trailing `/`.
fn normal(path: &str) -> String {
    let path = match path.strip_prefix(r"\\?\UNC\") {
        Some(share) => format!(r"\\{share}"),
        None => path.strip_prefix(r"\\?\").unwrap_or(path).to_string(),
    };
    let path = path.replace('\\', "/");
    match path.trim_end_matches('/') {
        "" if path.starts_with('/') => "/".to_string(),
        trimmed => trimmed.to_string(),
    }
}

/// Whether `path` (read by [`normal`]) is rooted: `/…` or a drive, `C:/…`.
fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || drive(path).is_some()
}

/// The drive letter `path` starts with, `C:`, if any.
fn drive(path: &str) -> Option<char> {
    let mut chars = path.chars();
    let letter = chars.next().filter(char::is_ascii_alphabetic)?;
    (chars.next() == Some(':')).then_some(letter)
}

/// `path`, and its other spelling under macOS's `/private` alias.
fn aliases(path: &str) -> Vec<String> {
    let mut spellings = vec![path.to_string()];
    for (long, short) in [("/private/var", "/var"), ("/private/tmp", "/tmp")] {
        for (from, to) in [(long, short), (short, long)] {
            match path.strip_prefix(from) {
                Some(rest) if rest.is_empty() || rest.starts_with('/') => {
                    spellings.push(format!("{to}{rest}"));
                }
                _ => {}
            }
        }
    }
    spellings
}

/// `path` at or below `root`, both read by [`normal`]: what follows `root/`, empty for `root` itself, or `None`.
fn under(path: &str, root: &str) -> Option<String> {
    let (path, root) = match (drive(path), drive(root)) {
        (Some(a), Some(b)) if a.eq_ignore_ascii_case(&b) => (&path[2..], &root[2..]),
        (None, None) => (path, root),
        _ => return None,
    };
    let rest = path.strip_prefix(root.trim_end_matches('/'))?;
    match rest.strip_prefix('/') {
        Some(rest) => Some(rest.to_string()),
        None => rest.is_empty().then(String::new),
    }
}
