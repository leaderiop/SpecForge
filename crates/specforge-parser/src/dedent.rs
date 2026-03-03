/// Strip common leading whitespace from a triple-quoted string (Kotlin-style trimMargin).
///
/// Given `"""\\n  line1\\n  line2\\n"""`, strips the `"""` delimiters and removes
/// the common leading whitespace from all non-empty lines.
pub fn dedent_triple_quoted(raw: &str) -> String {
    // Strip the triple-quote delimiters
    let inner = raw
        .strip_prefix("\"\"\"")
        .and_then(|s| s.strip_suffix("\"\"\""))
        .unwrap_or(raw);

    let lines: Vec<&str> = inner.lines().collect();

    // Find minimum indentation across non-empty lines (skip first line if blank)
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Strip leading/trailing blank lines and dedent
    let mut result_lines: Vec<&str> = Vec::new();
    for line in &lines {
        if line.trim().is_empty() && result_lines.is_empty() {
            // Skip leading blank lines
            continue;
        }
        if line.len() >= min_indent {
            result_lines.push(&line[min_indent..]);
        } else {
            result_lines.push(line.trim());
        }
    }

    // Remove trailing blank lines
    while result_lines.last().is_some_and(|l| l.trim().is_empty()) {
        result_lines.pop();
    }

    result_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_dedent() {
        let input = "\"\"\"\n    line one\n    line two\n  \"\"\"";
        // min indent = 4 (content lines), but """  is at indent 2 — doesn't count as it's trimmed
        // Actually closing """ is stripped. The content lines have indent 4, so stripped to 0.
        assert_eq!(dedent_triple_quoted(input), "line one\nline two");
    }

    #[test]
    fn uniform_indent() {
        let input = "\"\"\"\n  a\n  b\n  c\n\"\"\"";
        assert_eq!(dedent_triple_quoted(input), "a\nb\nc");
    }

    #[test]
    fn preserves_relative_indent() {
        let input = "\"\"\"\n    outer\n      inner\n    outer again\n  \"\"\"";
        // min indent = 4 (the outer lines), so strip 4 from all
        assert_eq!(
            dedent_triple_quoted(input),
            "outer\n  inner\nouter again",
        );
    }

    #[test]
    fn single_line() {
        let input = "\"\"\"hello world\"\"\"";
        assert_eq!(dedent_triple_quoted(input), "hello world");
    }

    #[test]
    fn empty() {
        let input = "\"\"\"\"\"\"";
        assert_eq!(dedent_triple_quoted(input), "");
    }
}
