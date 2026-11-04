//! The path globs a hook declares (ADR 0292) and a pass or command reads (ADR 0293): project-relative,
//! `/`-separated; `*` is any run of characters within a segment, `?` one character, and a segment `**` any number of
//! segments (none included).

/// Whether `path` matches `pattern`. The pattern may come from a project's own values, so the time is bounded by the
/// product of the two lengths, however many wildcards it holds.
pub fn matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').filter(|s| *s != ".").collect();
    let path: Vec<&str> = path.split('/').filter(|s| *s != ".").collect();
    wild(
        &pattern,
        &path,
        |word| *word == "**",
        |word, text| segment(word, text),
    )
}

fn segment(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    wild(&pattern, &text, |c| *c == '*', |c, t| *c == '?' || c == t)
}

/// Whether `pattern` matches all of `text`, where `any` is a run of any length (none included) and `one` says whether
/// an element matches one of `text`'s. Only the last `any` is ever gone back to: one that matched earlier can match
/// no more than it did, so going back further finds nothing new.
fn wild<P, T>(
    pattern: &[P],
    text: &[T],
    any: impl Fn(&P) -> bool,
    one: impl Fn(&P, &T) -> bool,
) -> bool {
    let (mut at, mut read) = (0, 0);
    let mut back: Option<(usize, usize)> = None;
    while read < text.len() {
        match pattern.get(at) {
            Some(element) if any(element) => {
                back = Some((at + 1, read));
                at += 1;
            }
            Some(element) if one(element, &text[read]) => {
                at += 1;
                read += 1;
            }
            _ => {
                let Some((after, tried)) = back else {
                    return false;
                };
                back = Some((after, tried + 1));
                at = after;
                read = tried + 1;
            }
        }
    }
    pattern[at..].iter().all(any)
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

    #[test]
    fn a_run_of_double_stars_is_one() {
        let pattern = "**/".repeat(40) + "x";
        let path = "a/".repeat(30) + "y";
        assert!(!matches(&pattern, &path));
        assert!(matches(&pattern, &("a/".repeat(30) + "x")));
    }

    #[test]
    fn a_hostile_pattern_is_answered_in_time() {
        let stars = "*a".repeat(64) + "b";
        assert!(!matches(&stars, &"a".repeat(4096)));
        let spread = "**/a/".repeat(64) + "b";
        assert!(!matches(&spread, &"a/".repeat(2048)));
    }

    #[test]
    fn wildcards_match_as_their_grammar_says() {
        assert!(matches("a*b*c", "axxbxxc"));
        assert!(!matches("a*b*c", "axxbxx"));
        assert!(matches("**/x/**/y", "x/y"));
        assert!(matches("**/x/**/y", "a/x/b/c/y"));
        assert!(!matches("**/x/**/y", "a/y/x"));
        assert!(matches("*", ""));
        assert!(!matches("?", ""));
        assert!(matches("é?", "éa"));
    }
}
