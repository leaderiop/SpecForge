//! The code of an asset's markdown: what a shell runs, as the scans over the assets read it.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

/// The code lines of `lines` (an asset's lines after its header) as `(index, trimmed line)`: the lines inside a fence
/// that are not a `#` comment. Prose about a command is not code.
pub fn fenced_code(lines: &[String]) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    // the open fence's character: a ``` fence is closed by ```, a ~~~ fence by ~~~
    let mut fence: Option<char> = None;
    for (n, line) in lines.iter().enumerate() {
        let code = line.trim();
        if code.starts_with("```") || code.starts_with("~~~") {
            let mark = code.chars().next();
            fence = match fence {
                None => mark,
                Some(open) if Some(open) == mark => None,
                open => open,
            };
            continue;
        }
        if fence.is_some() && !code.starts_with('#') {
            out.push((n, code));
        }
    }
    out
}

/// The code of `lines` (an asset's lines after its header) as `(index, text)`: each line inside a fence, and the
/// inline code spans of a line outside one.
pub fn code_of(lines: &[String]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut fence: Option<char> = None;
    for (n, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let mark = trimmed.chars().next();
            fence = match fence {
                None => mark,
                Some(open) if Some(open) == mark => None,
                open => open,
            };
            continue;
        }
        if fence.is_some() {
            out.push((n, line.clone()));
        } else {
            let spans: Vec<&str> = line.split('`').skip(1).step_by(2).collect();
            if !spans.is_empty() {
                out.push((n, spans.join(" ")));
            }
        }
    }
    out
}
