use super::*;

fn one(line: &str) -> Vec<String> {
    let mut c = commands(line).unwrap();
    assert_eq!(c.len(), 1, "{c:?}");
    c.remove(0)
}

#[test]
fn shell_quotes_group_words_and_escapes_keep_characters() {
    assert_eq!(
        one(r#"git commit -m "feat(03): a \"b\"" 'it''s'"#),
        ["git", "commit", "-m", "feat(03): a \"b\"", "its"]
    );
    assert_eq!(one(r"a\ b c"), ["a b", "c"]);
    assert_eq!(one(r#"echo "$HOME" ''"#), ["echo", "$HOME", ""]);
}

#[test]
fn shell_operators_end_a_command() {
    assert_eq!(
        commands("cd x && git commit -m y; echo done | cat").unwrap(),
        [
            vec!["cd", "x"],
            vec!["git", "commit", "-m", "y"],
            vec!["echo", "done"],
            vec!["cat"]
        ]
    );
    assert_eq!(commands("echo 'a;b'").unwrap(), [vec!["echo", "a;b"]]);
}

#[test]
fn shell_segments_name_the_operator_before_each_command() {
    let after: Vec<Option<Op>> = segments("a && b || c; d | e & f (g) h")
        .unwrap()
        .into_iter()
        .map(|segment| segment.after)
        .collect();
    assert_eq!(
        after,
        [
            None,
            Some(Op::And),
            Some(Op::Or),
            Some(Op::Semi),
            Some(Op::Pipe),
            Some(Op::Background),
            Some(Op::Open),
            Some(Op::Close),
        ]
    );
    let quoted = segments(r#"echo 'a && b' "c || d""#).unwrap();
    assert_eq!(quoted.len(), 1);
    assert_eq!(quoted[0].words, ["echo", "a && b", "c || d"]);
    let newline = segments("a\nb").unwrap();
    assert_eq!(newline[1].after, Some(Op::Newline));
}

#[test]
fn a_redirect_is_no_boundary_and_two_operators_keep_their_order() {
    assert_eq!(commands("make 2>&1").unwrap(), [vec!["make", "2>&1"]]);
    assert_eq!(
        commands("make &>log; ls").unwrap(),
        [vec!["make", "&>log"], vec!["ls"]]
    );
    assert_eq!(commands("make '>'&").unwrap(), [vec!["make", ">"]]);
    let after: Vec<Option<Op>> = segments("(a) && b")
        .unwrap()
        .into_iter()
        .map(|segment| segment.after)
        .collect();
    assert_eq!(after, [Some(Op::Open), Some(Op::Close), Some(Op::And)]);
}

#[test]
fn shell_refuses_an_unclosed_quote() {
    assert!(commands("echo 'a").is_err());
    assert!(commands("echo \"a").is_err());
    assert!(commands("echo a\\").is_err());
}
