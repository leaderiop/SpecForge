//! `composeWorkflow` (upstream's install, ADR-1671): a workflow is installed without its `<!-- gsd:section id=".."
//! when=".." -->` and `<!-- /gsd:section -->` marker lines, on every runtime and before any other rewrite. Only files
//! under `gsd-core/workflows/` are composed. The sections themselves stay: the workflow's own text says which to read.
//!
//! Also ports gsd-core bin/lib modules: context-predicates.cjs, prompt-budget.cjs.

/// The text of the embedded file `path`, as upstream installs it: workflows lose their section marker lines.
pub fn composed(path: &str, text: &str) -> String {
    if path.starts_with("gsd-core/workflows/") {
        strip_markers(text)
    } else {
        text.to_string()
    }
}

fn is_marker(line: &str) -> bool {
    let t = line.trim();
    t == "<!-- /gsd:section -->"
        || (t.starts_with("<!-- gsd:section ")
            && t.ends_with("-->")
            && t.matches("-->").count() == 1)
}

/// `text` without its marker lines (a line and its terminator). A marker inside a code fence or another HTML comment
/// is text, not structure.
fn strip_markers(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fence: Option<String> = None;
    let mut comment = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if let Some(open) = &fence {
            if t.starts_with(open.as_str()) && t.trim_end().chars().all(|c| c == '`' || c == '~') {
                fence = None;
            }
        } else if comment {
            comment = !line.contains("-->");
        } else if is_marker(line) {
            continue;
        } else if t.starts_with("```") || t.starts_with("~~~") {
            let ch = &t[..1];
            let run = t.chars().take_while(|c| c.to_string() == ch).count();
            fence = Some(ch.repeat(run));
        } else if t.starts_with("<!--") && !line.contains("-->") {
            comment = true;
        }
        out.push_str(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{composed, strip_markers};

    #[test]
    fn marker_lines_go_and_their_sections_stay() {
        let text =
            "a\n<!-- gsd:section id=\"x\" when=\"flag:--w\" -->\nbody\n<!-- /gsd:section -->\nz\n";
        assert_eq!(strip_markers(text), "a\nbody\nz\n");
    }

    #[test]
    fn a_marker_in_a_fence_or_a_comment_is_text() {
        let text = "```\n<!-- /gsd:section -->\n```\n<!--\n<!-- /gsd:section -->\n-->\n";
        assert_eq!(strip_markers(text), text);
    }

    #[test]
    fn only_workflows_are_composed() {
        let text = "<!-- /gsd:section -->\n";
        assert_eq!(composed("gsd-core/workflows/x.md", text), "");
        assert_eq!(composed("gsd-core/references/x.md", text), text);
    }
}
