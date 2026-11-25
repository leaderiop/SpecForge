//! nextest's JUnit reports, merged into the one report `@rust/test` reads (ADR 0374 D5).
//!
//! CI's test partitions each write their own `junit.xml`; `specforge collect --runner rust-test` takes one report.
//! The merge splices them as text, with no XML parser: every `<testsuite>` of each report, in order, under one
//! `<testsuites name="merged">` root.

#[cfg(test)]
mod tests;

/// `reports` (each a nextest JUnit document) as one: an XML declaration, `<testsuites name="merged">`, the inside
/// of each report's `<testsuites>` root in order, `</testsuites>`. An empty list is an error naming the run that
/// writes the reports; a report with no `<testsuites>` root is an error naming its index.
pub fn merge(reports: &[String]) -> Result<String, String> {
    if reports.is_empty() {
        return Err(
            "no JUnit report to merge: nextest writes one under its `gate` and `ci` profiles \
             (run `cargo gate full --only tests,guests,gsd,dogfood`)"
                .to_string(),
        );
    }
    let mut merged =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites name=\"merged\">\n");
    for (index, report) in reports.iter().enumerate() {
        let inside = inside_root(report).ok_or_else(|| {
            format!(
                "report {} of {} has no <testsuites> root: it is not a nextest JUnit report",
                index + 1,
                reports.len()
            )
        })?;
        if !inside.is_empty() {
            merged.push_str(inside);
            merged.push('\n');
        }
    }
    merged.push_str("</testsuites>\n");
    Ok(merged)
}

/// The text between the end of `report`'s `<testsuites …>` start tag and its `</testsuites>`, without the blank lines
/// around it; `None` when there is no such root. A self-closed `<testsuites …/>` holds nothing.
fn inside_root(report: &str) -> Option<&str> {
    let open = report.find("<testsuites")?;
    let tag_end = open + report[open..].find('>')?;
    if report[..tag_end].ends_with('/') {
        return Some("");
    }
    let body = &report[tag_end + 1..];
    let close = body.rfind("</testsuites>")?;
    Some(trim_blank_lines(&body[..close]))
}

/// `text` without its leading and trailing blank lines; the indentation of its first line is kept.
fn trim_blank_lines(text: &str) -> &str {
    let end = text.trim_end().len();
    let text = &text[..end];
    let start = text
        .char_indices()
        .take_while(|(_, c)| c.is_whitespace())
        .filter(|(_, c)| *c == '\n')
        .map(|(i, _)| i + 1)
        .last()
        .unwrap_or(0);
    &text[start..]
}
