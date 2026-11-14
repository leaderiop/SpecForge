//! The code of an asset's markdown: what a shell runs, as the scans over the assets read it. gsd-kit's Markdown
//! reader finds the fences and code spans for every scan.
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

/// Each line of `lines` that is no fence mark, as `(index, line, where it stands)`, as gsd-kit's Markdown reader
/// reads the lines joined with `\n`.
pub fn walk(lines: &[String]) -> Vec<(usize, &str, At<'_>)> {
    let mut at = vec![Some(At::Prose); lines.len()];
    for (n, fence) in gsd_kit::markdown::fences(&lines.join("\n"))
        .iter()
        .enumerate()
    {
        let open = lines[fence.open].as_str();
        // the info string's first word, as the opening line spells it
        let language = open
            .find(fence.language.as_str())
            .map_or("", |i| &open[i..i + fence.language.len()]);
        at[fence.open] = None;
        if fence.closed {
            at[fence.code.end] = None;
        }
        for line in fence.code.clone() {
            at[line] = Some(At::Fence { n, language });
        }
    }
    lines
        .iter()
        .zip(at)
        .enumerate()
        .filter_map(|(n, (line, at))| Some((n, line.as_str(), at?)))
        .collect()
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
/// inline code spans that start on a line outside one, joined with a space. A span may cross lines.
pub fn code_of(lines: &[String]) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = walk(lines)
        .into_iter()
        .filter(|(_, _, at)| *at != At::Prose)
        .map(|(n, line, _)| (n, line.to_string()))
        .collect();
    for (n, span) in gsd_kit::markdown::inline_code(&lines.join("\n")) {
        match out.iter_mut().find(|(at, _)| *at == n) {
            Some((_, spans)) => {
                spans.push(' ');
                spans.push_str(&span);
            }
            None => out.push((n, span)),
        }
    }
    out.sort_by_key(|(n, _)| *n);
    out
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
