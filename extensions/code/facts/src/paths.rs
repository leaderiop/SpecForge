//! Project-relative paths as the packs compute them from what a manifest or a source file writes.

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
