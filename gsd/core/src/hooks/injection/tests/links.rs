//! The link rules' line-by-line shortcut in `link_findings`.

use super::super::patterns::LINK_RULES;
use super::super::regex::{Node, parse};

/// `link_findings` runs a rule line by line only when the whole text matches it: sound while no link rule can match a
/// line without matching the text, so none has `^` or a look-around.
#[test]
fn a_link_rule_that_matches_a_line_matches_the_text() {
    fn anchored(n: &Node) -> bool {
        match n {
            Node::Start | Node::Look { .. } => true,
            Node::Seq(v) | Node::Alt(v) => v.iter().any(anchored),
            Node::Rep { node, .. } => anchored(node),
            Node::Char(_) | Node::Space | Node::Class { .. } => false,
        }
    }
    for (id, p) in LINK_RULES {
        assert!(!anchored(&parse(p)), "{id}: {p}");
    }
}
