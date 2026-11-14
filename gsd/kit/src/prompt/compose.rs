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

/// `text` without its marker lines (a line and its terminator): a marker line on which an HTML block starts, as the
/// gsd family's Markdown reader reads it. A marker in a fence is code, and one inside a comment is text.
fn strip_markers(text: &str) -> String {
    let starts: Vec<usize> = crate::markdown::html_blocks(text)
        .into_iter()
        .map(|block| block.start)
        .collect();
    text.split_inclusive('\n')
        .enumerate()
        .filter(|(l, line)| !(is_marker(line) && starts.contains(l)))
        .map(|(_, line)| line)
        .collect()
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
    fn a_marker_under_a_section_tag_goes() {
        let text = "<step name=\"x\">\n<!-- gsd:section id=\"a\" when=\"flag:--w\" -->\nbody\n```\n<!-- /gsd:section -->\n```\n<!-- /gsd:section -->\n</step>\n";
        assert_eq!(
            strip_markers(text),
            "<step name=\"x\">\nbody\n```\n<!-- /gsd:section -->\n```\n</step>\n"
        );
    }

    #[test]
    fn only_workflows_are_composed() {
        let text = "<!-- /gsd:section -->\n";
        assert_eq!(composed("gsd-core/workflows/x.md", text), "");
        assert_eq!(composed("gsd-core/references/x.md", text), text);
    }
}
