//! The XML-ish elements of a GSD file (`<objective>`, `<task type="auto">`, `<action>`), cut by their literal closing
//! tag (13-RESEARCH Pitfall 2): a body holds `<`, quotes and code that no XML reader would take, so the body is the text
//! up to the first `</name>` after the opening tag. No XML crate (ADR 0391 Rejected). A tag on a line of a fenced code
//! block is not a tag.
//!
//! Lines are 1-based, as a report entry gives them.

use gsd_kit::markdown;

use crate::frontmatter::Refused;

/// One element: its attributes as written (`type="auto"`), its body verbatim, and the 1-based line of its opening tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tagged {
    pub attrs: Vec<(String, String)>,
    pub body: String,
    pub line: usize,
}

/// Every `<name ...>` element of `text` outside a fence, in order, and what was refused: an opening tag whose attributes
/// are not `key="value"`, and an unclosed element, which ends the reading (all that follows it would be its body).
pub fn tagged(text: &str, name: &str) -> (Vec<Tagged>, Vec<Refused>) {
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let line = |offset: usize| starts.partition_point(|&s| s <= offset);
    let mut fenced = vec![false; starts.len()];
    for fence in markdown::fences(text) {
        let end = (fence.code.end + 1).min(fenced.len());
        fenced[fence.open.min(end)..end].fill(true);
    }
    let (open, close) = (format!("<{name}"), format!("</{name}>"));
    let (mut found, mut refused) = (Vec::new(), Vec::new());
    let mut from = 0;
    while let Some(at) = text[from..].find(&open).map(|i| from + i) {
        let after = at + open.len();
        from = after;
        let no = line(at);
        if fenced[no - 1] || !text[after..].starts_with([' ', '\t', '\n', '>', '/']) {
            continue;
        }
        let Some(gt) = tag_end(&text[after..]) else {
            refused.push(Refused {
                line: no,
                reason: format!("<{name}> has no `>` ending its opening tag"),
            });
            break;
        };
        let inside = &text[after..after + gt];
        from = after + gt + 1;
        let (inside, empty) = match inside.trim_end().strip_suffix('/') {
            Some(inside) => (inside, true),
            None => (inside, false),
        };
        let Some(attrs) = attributes(inside) else {
            refused.push(Refused {
                line: no,
                reason: format!("<{name}>'s attributes are not key=\"value\""),
            });
            continue;
        };
        if empty {
            found.push(Tagged {
                attrs,
                body: String::new(),
                line: no,
            });
            continue;
        }
        match text[from..].find(&close) {
            Some(end) => {
                found.push(Tagged {
                    attrs,
                    body: text[from..from + end].to_string(),
                    line: no,
                });
                from += end + close.len();
            }
            None => {
                refused.push(Refused {
                    line: no,
                    reason: format!("<{name}> is not closed by {close}"),
                });
                break;
            }
        }
    }
    (found, refused)
}

/// The offset of the `>` ending an opening tag, outside a quoted attribute value.
fn tag_end(text: &str) -> Option<usize> {
    let mut quoted = false;
    for (i, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '>' if !quoted => return Some(i),
            _ => {}
        }
    }
    None
}

/// `key="value"` pairs separated by white space; none when the text is anything else.
fn attributes(mut text: &str) -> Option<Vec<(String, String)>> {
    let mut attrs = Vec::new();
    loop {
        text = text.trim_start();
        if text.is_empty() {
            return Some(attrs);
        }
        let (key, rest) = text.split_once("=\"")?;
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b':')
        {
            return None;
        }
        let (value, rest) = rest.split_once('"')?;
        attrs.push((key.to_string(), value.to_string()));
        text = rest;
        if !(text.is_empty() || text.starts_with([' ', '\t', '\n'])) {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_keeps_its_angle_brackets_and_quotes() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../parity/fixtures/taskboard/.planning/phases/01-core/01-02-PLAN.md"
        ))
        .expect("the fixture");
        let (actions, refused) = tagged(&text, "action");
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(actions.len(), 2);
        assert!(
            actions[0]
                .body
                .contains("throw `no task <id>` when it is missing"),
            "{}",
            actions[0].body
        );
        assert!(
            actions[1]
                .body
                .contains("`add <title>` prints `added <id>: <title>`")
        );
        let (verify, _) = tagged(&text, "verify");
        assert_eq!(verify[1].body, "node src/cli.js add \"Write the roadmap\"");
        let (tasks, _) = tagged(&text, "task");
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].attrs, [("type".to_string(), "auto".to_string())]);
        assert_eq!(actions[0].line, tasks[0].line + 3);
    }

    #[test]
    fn an_opening_tag_may_carry_attributes_and_a_longer_name_is_not_it() {
        let text = "<tasks>\n<task type=\"checkpoint:human-verify\" gate=\"blocking\">\n<name>x</name>\n</task>\n</tasks>\n";
        let (tasks, refused) = tagged(text, "task");
        assert!(refused.is_empty());
        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks[0].attrs,
            [
                ("type".to_string(), "checkpoint:human-verify".to_string()),
                ("gate".to_string(), "blocking".to_string())
            ]
        );
        assert_eq!(
            (tasks[0].line, tasks[0].body.as_str()),
            (2, "\n<name>x</name>\n")
        );
    }

    #[test]
    fn an_unclosed_tag_is_one_refusal_and_no_body() {
        let text = "<objective>\nok\n</objective>\n\n<action>\nnever closed <action> again\n";
        let (found, _) = tagged(text, "objective");
        assert_eq!(found.len(), 1);
        let (actions, refused) = tagged(text, "action");
        assert!(actions.is_empty());
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert_eq!(refused[0].line, 5);
    }

    #[test]
    fn a_tag_in_a_fence_is_not_a_tag() {
        let text = "Example:\n\n```xml\n<action>not this</action>\n```\n\n<action>this</action>\n";
        let (actions, refused) = tagged(text, "action");
        assert!(refused.is_empty());
        assert_eq!(actions.len(), 1);
        assert_eq!((actions[0].body.as_str(), actions[0].line), ("this", 7));
    }

    #[test]
    fn a_body_ends_at_the_first_literal_closing_tag() {
        let (found, refused) = tagged(
            "<verify>a</verify> b </verify>\n<verify>c</verify>",
            "verify",
        );
        assert!(refused.is_empty());
        let bodies: Vec<&str> = found.iter().map(|t| t.body.as_str()).collect();
        assert_eq!(bodies, ["a", "c"]);
    }

    #[test]
    fn malformed_attributes_are_refused_at_their_line() {
        let (found, refused) = tagged("\n<task type=auto>x</task>\n", "task");
        assert!(found.is_empty());
        assert_eq!(refused[0].line, 2);
    }
}
