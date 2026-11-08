//! The one scan that fills a template's tokens: `{{<kind>:<key>|<fallback>}}` (a setting, a typed argument) and
//! `$ARGUMENTS`.

const ARGUMENTS: &str = "$ARGUMENTS";
const CLOSE: &str = "}}";

/// A braced token kind: the word before its `:` (`setting`, `arg`) and what fills `(key, fallback)`, the key trimmed;
/// `None` keeps the token as written.
pub(crate) type Kind<'a> = (&'a str, &'a dyn Fn(&str, &str) -> Option<String>);

/// `text` with its tokens filled in one left-to-right pass: each braced token of a kind of `kinds`, and `$ARGUMENTS`
/// (not `$ARGUMENTS_X`) by `arguments` when it is given. What a fill writes is never scanned again, so a value that
/// holds a token stays data. A token without its `}}` ends the scan: the rest is written as it is.
pub(crate) fn fill(text: &str, kinds: &[Kind<'_>], arguments: Option<&str>) -> String {
    let opens: Vec<String> = kinds
        .iter()
        .map(|(kind, _)| format!("{{{{{kind}:"))
        .collect();
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        // the earliest token; `None` is `$ARGUMENTS`
        let braced = opens.iter().zip(kinds).filter_map(|(open, (_, fill))| {
            rest.find(open.as_str())
                .map(|at| (at, Some((open.len(), fill))))
        });
        let dollar = arguments
            .and_then(|_| rest.find(ARGUMENTS))
            .map(|at| (at, None));
        let Some((at, token)) = braced.chain(dollar).min_by_key(|(at, _)| *at) else {
            break;
        };
        out.push_str(&rest[..at]);
        let Some((open, fill)) = token else {
            let tail = &rest[at + ARGUMENTS.len()..];
            let word = tail
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
            out.push_str(if word {
                ARGUMENTS
            } else {
                arguments.unwrap_or_default()
            });
            rest = tail;
            continue;
        };
        let after = &rest[at + open..];
        let Some(end) = after.find(CLOSE) else {
            out.push_str(&rest[at..]);
            return out;
        };
        let (key, fallback) = after[..end].split_once('|').unwrap_or((&after[..end], ""));
        match fill(key.trim(), fallback) {
            Some(value) => out.push_str(&value),
            None => out.push_str(&rest[at..at + open + end + CLOSE.len()]),
        }
        rest = &after[end + CLOSE.len()..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::fill;

    fn upper(key: &str, fallback: &str) -> Option<String> {
        (key != "kept").then(|| format!("<{}|{fallback}>", key.to_uppercase()))
    }

    #[test]
    fn each_kind_is_filled_by_its_own_closure_with_its_key_trimmed() {
        let lower = |key: &str, _: &str| Some(key.to_string());
        let kinds: [super::Kind<'_>; 2] = [("a", &upper), ("b", &lower)];
        assert_eq!(
            fill("{{a: x |f}} {{b: Y }} {{a:kept|z}} {{c:x}}", &kinds, None),
            "<X|f> Y {{a:kept|z}} {{c:x}}"
        );
    }

    #[test]
    fn what_a_fill_writes_is_not_scanned_again() {
        let echo = |key: &str, _: &str| Some(format!("{{{{a:{key}}}}} $ARGUMENTS"));
        let kinds: [super::Kind<'_>; 1] = [("a", &echo)];
        assert_eq!(
            fill("{{a:x}} $ARGUMENTS $ARGUMENTS_X", &kinds, Some("{{a:y}}")),
            "{{a:x}} $ARGUMENTS {{a:y}} $ARGUMENTS_X"
        );
        assert_eq!(fill("$ARGUMENTS", &kinds, None), "$ARGUMENTS");
    }

    #[test]
    fn an_open_token_ends_the_scan() {
        let kinds: [super::Kind<'_>; 1] = [("a", &upper)];
        assert_eq!(
            fill("{{a:x}} {{a:y $ARGUMENTS", &kinds, Some("A")),
            "<X|> {{a:y $ARGUMENTS"
        );
    }
}
