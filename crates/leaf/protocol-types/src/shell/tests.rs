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
fn a_redirect_ends_the_word_before_it_and_is_no_part_of_the_command() {
    use RedirectOp::*;
    // (line, the command's words, each redirect's descriptor, operator and target)
    for (line, words, redirects) in [
        ("a b>c", vec!["a", "b"], vec![(None, Out, "c")]),
        ("a b >c", vec!["a", "b"], vec![(None, Out, "c")]),
        ("a b > c d", vec!["a", "b", "d"], vec![(None, Out, "c")]),
        ("a>>c b", vec!["a", "b"], vec![(None, Append, "c")]),
        ("a<c b", vec!["a", "b"], vec![(None, In, "c")]),
        ("a&>c b", vec!["a", "b"], vec![(None, Both, "c")]),
        ("a&>>c b", vec!["a", "b"], vec![(None, BothAppend, "c")]),
        ("a >|c b", vec!["a", "b"], vec![(None, Clobber, "c")]),
        ("a <>c b", vec!["a", "b"], vec![(None, ReadWrite, "c")]),
        ("a 2>c b", vec!["a", "b"], vec![(Some("2"), Out, "c")]),
        ("a 2>>c b", vec!["a", "b"], vec![(Some("2"), Append, "c")]),
        ("a 2>&1 b", vec!["a", "b"], vec![(Some("2"), DupOut, "1")]),
        ("a >&2 b", vec!["a", "b"], vec![(None, DupOut, "2")]),
        ("a 2>&- b", vec!["a", "b"], vec![(Some("2"), DupOut, "-")]),
        ("a <&3 b", vec!["a", "b"], vec![(None, DupIn, "3")]),
        ("a {fd}>c b", vec!["a", "b"], vec![(Some("{fd}"), Out, "c")]),
        (">c a b", vec!["a", "b"], vec![(None, Out, "c")]),
        ("2>c a b", vec!["a", "b"], vec![(Some("2"), Out, "c")]),
        ("&>c a b", vec!["a", "b"], vec![(None, Both, "c")]),
        (
            "a>c 2>&1 b<d",
            vec!["a", "b"],
            vec![(None, Out, "c"), (Some("2"), DupOut, "1"), (None, In, "d")],
        ),
        ("a '>' b", vec!["a", ">", "b"], vec![]),
        (r"a \> b", vec!["a", ">", "b"], vec![]),
        ("a '>'>c", vec!["a", ">"], vec![(None, Out, "c")]),
        (">'c d' a", vec!["a"], vec![(None, Out, "c d")]),
        // Digits are a descriptor only as a word of their own.
        ("a2>c", vec!["a2"], vec![(None, Out, "c")]),
        ("a '2'>c", vec!["a", "2"], vec![(None, Out, "c")]),
        (r"a \2>c", vec!["a", "2"], vec![(None, Out, "c")]),
        ("a 2 >c", vec!["a", "2"], vec![(None, Out, "c")]),
        ("a 2&>c", vec!["a", "2"], vec![(None, Both, "c")]),
        // A heredoc's tag and a here-string's text are no file.
        ("a <<T b", vec!["a", "b"], vec![]),
        ("a <<-T b", vec!["a", "b"], vec![]),
        ("a <<<'t u' b", vec!["a", "b"], vec![]),
    ] {
        let read = segments(line).unwrap();
        assert_eq!(read.len(), 1, "{line}: {read:?}");
        assert_eq!(read[0].words, words, "{line}");
        let read_redirects: Vec<(Option<&str>, RedirectOp, &str)> = read[0]
            .redirects
            .iter()
            .map(|r| (r.fd.as_deref(), r.op, r.target.as_str()))
            .collect();
        assert_eq!(read_redirects, redirects, "{line}");
        assert_eq!(read[0].after, None, "{line}");
    }
}

#[test]
fn a_redirect_is_no_boundary_and_two_operators_keep_their_order() {
    assert_eq!(commands("make 2>&1").unwrap(), [vec!["make"]]);
    assert_eq!(
        commands("make &>log; ls").unwrap(),
        [vec!["make"], vec!["ls"]]
    );
    assert_eq!(commands("make '>'&").unwrap(), [vec!["make", ">"]]);
    assert_eq!(commands(">x").unwrap(), Vec::<Vec<String>>::new());
    assert_eq!(commands("a &> x & b").unwrap(), [vec!["a"], vec!["b"]]);
    assert_eq!(
        words("specforge&>/dev/null gsd answer-checkpoint&>y").unwrap(),
        ["specforge", "gsd", "answer-checkpoint"]
    );
    let after: Vec<Option<Op>> = segments("(a) && b")
        .unwrap()
        .into_iter()
        .map(|segment| segment.after)
        .collect();
    assert_eq!(after, [Some(Op::Open), Some(Op::Close), Some(Op::And)]);
    // A process substitution is a command of its own, not a target.
    let read = segments("a <(b c) d").unwrap();
    assert_eq!(read[0].words, ["a"]);
    assert_eq!(read[1].words, ["b", "c"]);
    assert_eq!(read[1].after, Some(Op::Open));
    assert!(read[0].redirects.is_empty());
}

#[test]
fn a_substitution_inside_double_quotes_stays_in_its_word_and_is_read() {
    let read = segments(r#"echo "x$(cd a && b)y" z"#).unwrap();
    assert_eq!(read[0].words, ["echo", "x$(cd a && b)y", "z"]);
    assert_eq!(read[0].substitutions, ["cd a && b"]);
    assert_eq!(
        commands(r#"echo "$(cd a && b)""#).unwrap(),
        [vec!["cd", "a"], vec!["b"], vec!["echo", "$(cd a && b)"]]
    );
    assert_eq!(
        commands(r#"echo "`cd a`""#).unwrap(),
        [vec!["cd", "a"], vec!["echo", "`cd a`"]]
    );
    // Its quotes are its own, and one inside another is read too.
    assert_eq!(
        commands(r#"echo "$(cat "a b")" c"#).unwrap(),
        [vec!["cat", "a b"], vec!["echo", r#"$(cat "a b")"#, "c"]]
    );
    assert_eq!(
        commands(r#"echo "$(echo "$(cd a)")""#).unwrap(),
        [
            vec!["cd", "a"],
            vec!["echo", "$(cd a)"],
            vec!["echo", r#"$(echo "$(cd a)")"#]
        ]
    );
    // What the shell leaves as text is text.
    for line in [
        r#"echo '$(cd a)'"#,
        r#"echo "\$(cd a)""#,
        r#"echo "\`cd a\`""#,
        "echo '`cd a`'",
    ] {
        assert_eq!(commands(line).unwrap().len(), 1, "{line}");
    }
}

#[test]
fn a_bare_substitution_stays_in_its_word_and_is_read() {
    for (line, word) in [
        ("a `b c`/d e", "`b c`/d"),
        ("a $(b c)/d e", "$(b c)/d"),
        ("a x$(b c) e", "x$(b c)"),
    ] {
        assert_eq!(
            commands(line).unwrap(),
            [vec!["b", "c"], vec!["a", word, "e"]],
            "{line}"
        );
    }
    assert_eq!(
        commands("x=$(cd a; b)").unwrap(),
        [vec!["cd", "a"], vec!["b"], vec!["x=$(cd a; b)"]]
    );
    // Only a `(` after a `$` is one; a `$(` no `)` closes reads as the group it starts.
    assert_eq!(
        commands("a (b c) d").unwrap(),
        [vec!["a"], vec!["b", "c"], vec!["d"]]
    );
    assert_eq!(commands("a $(b").unwrap(), [vec!["a", "$"], vec!["b"]]);
    assert!(commands("a `b").is_err());
    assert!(commands(r#"echo "`b""#).is_err());
}

#[test]
fn a_text_handed_over_in_a_substitution_runs_nothing() {
    // Claude Code's commit idiom: its lines are a message, whatever they say, with or without a lone quote.
    for line in [
        "git commit -m \"$(cat <<'EOF'\nrun git add -f x and specforge t approve\nEOF\n)\"",
        "git commit -m \"$(cat <<'EOF'\nit's git add -f x\nEOF\n)\"",
        "git commit -m \"$(cat <<'EOF'\na (b\nEOF\n)\"",
        "git commit -m \"$(cat <<-EOF\nit's and it's\nEOF\n)\"",
    ] {
        let read = commands(line).unwrap();
        assert_eq!(read.len(), 1, "{line}: {read:?}");
        assert_eq!(read[0][..3], ["git", "commit", "-m"], "{line}");
    }
    // Any other command given the lines may run them.
    let read = commands("echo \"$(bash <<EOF\ncd a\nEOF\n)\"").unwrap();
    assert!(read.iter().any(|c| c == &["cd", "a"]), "{read:?}");
    let read = commands("echo \"$(cat <<EOF | sh\ncd a\nEOF\n)\"").unwrap();
    assert!(read.iter().any(|c| c == &["cd", "a"]), "{read:?}");
}

#[test]
fn shell_refuses_an_unclosed_quote() {
    assert!(commands("echo 'a").is_err());
    assert!(commands("echo \"a").is_err());
    assert!(commands("echo a\\").is_err());
}

#[test]
fn a_hostile_nest_or_a_long_line_stays_bounded() {
    // Past the bound a nest is refused, in double quotes or bare, and never takes the stack.
    let quoted = format!("echo \"{}x{}\"", "$(echo \"".repeat(100), "\")".repeat(100));
    assert!(commands(&quoted).is_ok(), "past the bound it reads as text");
    let bare = format!("{}x{}", "$(".repeat(5000), ")".repeat(5000));
    assert!(commands(&bare).is_err());
    // A `$(` that nothing closes takes one scan of the line, however many there are.
    let open = format!("\"{}\"", "$( ".repeat(200_000));
    let started = std::time::Instant::now();
    assert!(commands(&open).is_ok());
    assert!(started.elapsed().as_secs() < 30, "{:?}", started.elapsed());
}

#[test]
fn a_redirect_keeps_its_operator_and_descriptor() {
    use RedirectOp::*;
    for (line, redirects) in [
        ("a <x", vec![(None, In, "x")]),
        ("a >x", vec![(None, Out, "x")]),
        ("a 2>>x", vec![(Some("2"), Append, "x")]),
        ("a >|x", vec![(None, Clobber, "x")]),
        ("a 2>&1", vec![(Some("2"), DupOut, "1")]),
        ("a >&log", vec![(None, Both, "log")]),
        ("a &>>x", vec![(None, BothAppend, "x")]),
        ("a <>x", vec![(None, ReadWrite, "x")]),
        ("a {fd}<x", vec![(Some("{fd}"), In, "x")]),
        ("a <<T", vec![]),
    ] {
        let read = segments(line).unwrap();
        let got: Vec<(Option<&str>, RedirectOp, &str)> = read[0]
            .redirects
            .iter()
            .map(|r| (r.fd.as_deref(), r.op, r.target.as_str()))
            .collect();
        assert_eq!(got, redirects, "{line}");
    }
    let read = segments(r#"while read l; do :; done < "$P/03-CONTEXT.md""#).unwrap();
    let redirects: Vec<&Redirect> = read.iter().flat_map(|s| &s.redirects).collect();
    assert_eq!(redirects.len(), 1, "{read:?}");
    assert_eq!(redirects[0].op, In);
    assert_eq!(redirects[0].target, "$P/03-CONTEXT.md");
}

#[test]
fn only_an_output_redirect_writes() {
    use RedirectOp::*;
    for (op, writes) in [
        (In, false),
        (DupIn, false),
        (DupOut, false),
        (ReadWrite, true),
        (Out, true),
        (Append, true),
        (Clobber, true),
        (Both, true),
        (BothAppend, true),
    ] {
        assert_eq!(op.writes(), writes, "{op:?}");
    }
}
