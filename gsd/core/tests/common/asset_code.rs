//! The code of an asset's markdown: what a shell runs, as the scans over the assets read it. One walk reads the fences
//! for every scan.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

/// Where a line of an asset's markdown stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum At<'a> {
    /// Outside every fence.
    Prose,
    /// In the asset's `n`th fence (from 0), of `language` (`bash`, or empty).
    Fence { n: usize, language: &'a str },
}

/// Each line of `lines` that is no fence mark, as `(index, line, where it stands)`. A ``` fence is closed by ```, a
/// ~~~ fence by ~~~.
pub fn walk(lines: &[String]) -> Vec<(usize, &str, At<'_>)> {
    let mut out = Vec::new();
    let mut open: Option<(char, &str)> = None;
    let mut fences = 0;
    for (n, line) in lines.iter().enumerate() {
        let code = line.trim();
        if code.starts_with("```") || code.starts_with("~~~") {
            let mark = code.chars().next().unwrap_or('`');
            open = match open {
                None => {
                    fences += 1;
                    Some((mark, code[3..].trim()))
                }
                Some((was, _)) if was == mark => None,
                open => open,
            };
            continue;
        }
        let at = match open {
            None => At::Prose,
            Some((_, language)) => At::Fence {
                n: fences - 1,
                language,
            },
        };
        out.push((n, line.as_str(), at));
    }
    out
}

/// The code lines of `lines` (an asset's lines after its header) as `(index, trimmed line)`: the lines inside a fence
/// that are not a `#` comment. Prose about a command is not code.
pub fn fenced_code(lines: &[String]) -> Vec<(usize, &str)> {
    walk(lines)
        .into_iter()
        .filter(|(_, _, at)| *at != At::Prose)
        .map(|(n, line, _)| (n, line.trim()))
        .filter(|(_, code)| !code.starts_with('#'))
        .collect()
}

/// The code of `lines` (an asset's lines after its header) as `(index, text)`: each line inside a fence, and the
/// inline code spans of a line outside one.
pub fn code_of(lines: &[String]) -> Vec<(usize, String)> {
    walk(lines)
        .into_iter()
        .filter_map(|(n, line, at)| match at {
            At::Fence { .. } => Some((n, line.to_string())),
            At::Prose => {
                let spans: Vec<&str> = line.split('`').skip(1).step_by(2).collect();
                (!spans.is_empty()).then(|| (n, spans.join(" ")))
            }
        })
        .collect()
}

/// Every markdown asset under `dir` (from the crate's root), as `(its path under `dir`, its lines after the header)`.
pub fn assets_under(dir: &str) -> Vec<(String, Vec<String>)> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    super::files::under(&base)
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p| {
            let relative = p.strip_prefix(&base).expect("under its base");
            let text = std::fs::read_to_string(p).expect("an asset");
            let lines = text.split('\n').skip(1).map(String::from).collect();
            (relative.to_string_lossy().into_owned(), lines)
        })
        .collect()
}
