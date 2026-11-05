//! Text the render writes into a shell line: the one escaping a setting's and a typed argument's value take.

/// `raw` as shell text inside double quotes: `"`, `\`, `$` and a backtick are escaped so a value can never end the
/// quote or run a command.
pub fn in_double_quotes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::in_double_quotes;

    #[test]
    fn a_value_cannot_leave_its_double_quotes() {
        assert_eq!(
            in_double_quotes("a\"b\\c $(x) `y` 'z'"),
            "a\\\"b\\\\c \\$(x) \\`y\\` 'z'"
        );
    }
}
