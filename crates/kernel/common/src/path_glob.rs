//! The path globs a hook declares (ADR 0292) and a pass or command reads (ADR 0293): project-relative,
//! `/`-separated; `*` is any run of characters within a segment, `?` one character, and a segment `**` any number of
//! segments (none included).

/// Whether `path` matches `pattern`.
pub fn matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').filter(|s| *s != ".").collect();
    let path: Vec<&str> = path.split('/').filter(|s| *s != ".").collect();
    segments(&pattern, &path)
}

fn segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|skip| segments(rest, &path[skip..])),
        Some((first, rest)) => path
            .split_first()
            .is_some_and(|(head, tail)| segment(first, head) && segments(rest, tail)),
    }
}

fn segment(pattern: &str, text: &str) -> bool {
    fn go(p: &[char], t: &[char]) -> bool {
        match p.split_first() {
            None => t.is_empty(),
            Some(('*', rest)) => (0..=t.len()).any(|n| go(rest, &t[n..])),
            Some(('?', rest)) => t.split_first().is_some_and(|(_, tail)| go(rest, tail)),
            Some((c, rest)) => t
                .split_first()
                .is_some_and(|(h, tail)| h == c && go(rest, tail)),
        }
    }
    go(
        &pattern.chars().collect::<Vec<_>>(),
        &text.chars().collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn glob_matches_by_segment() {
        assert!(matches("**", "a/b/c.spec"));
        assert!(matches("**", "c.spec"));
        assert!(matches("specs/**/*.spec", "specs/a.spec"));
        assert!(matches("specs/**/*.spec", "specs/x/y/a.spec"));
        assert!(!matches("specs/**/*.spec", "other/a.spec"));
        assert!(!matches("*.spec", "a/b.spec"));
        assert!(matches("a?.md", "ab.md"));
        assert!(matches("./a/*", "a/b"));
    }
}
