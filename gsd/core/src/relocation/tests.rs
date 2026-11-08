use super::*;

fn cds(line: &str) -> Vec<String> {
    events(line)
        .expect("readable")
        .into_iter()
        .filter_map(|event| match event {
            Event::Cd(target) => Some(target),
            _ => None,
        })
        .collect()
}

/// The cwd the line ends in, as relative moves joined; a subshell's move does not count.
fn reached(line: &str) -> Vec<String> {
    let mut stack: Vec<Vec<String>> = Vec::new();
    let mut now: Vec<String> = Vec::new();
    for event in events(line).expect("readable") {
        match event {
            Event::Enter => stack.push(now.clone()),
            Event::Leave => now = stack.pop().expect("balanced"),
            Event::Cd(target) => now.push(target),
            _ => {}
        }
    }
    assert!(stack.is_empty(), "balanced: {line}");
    now
}

#[test]
fn launchers_and_keywords_are_seen_through() {
    for line in [
        "cd x",
        "if cd x; then make; fi",
        "! cd x",
        "time cd x",
        "{ cd x; }",
        "FOO=1 cd x",
        "command cd x",
        "builtin cd x",
        "exec cd x",
        "/usr/bin/env cd x",
        "env -u FOO --debug -u BAR cd x",
        "env --unset=FOO cd x",
        "env -i - FOO=1 cd x",
        "timeout --signal TERM 10 cd x",
        "timeout -k 5 10s cd x",
        "nohup cd x",
        "stdbuf -oL -e L cd x",
        "nice -n 5 cd x",
        "ionice -c 2 -n 3 cd x",
        "sudo -u me cd x",
        "env -S 'cd x'",
    ] {
        assert_eq!(cds(line), ["x"], "{line}");
    }
    for line in [
        "time -p cd x",
        "time -o log cd x",
        "env -iS 'cd x'",
        "env -vS'cd x'",
    ] {
        assert_eq!(cds(line), ["x"], "{line}");
    }
    // A launcher's directory moves its payload.
    for line in [
        "env -iC d make",
        "env -iCd make",
        "env -i -C d make",
        "env -C d make",
        "env --chdir d make",
        "env --chdir=d make",
        "env -Cd make",
        "sudo -D d make",
    ] {
        assert_eq!(cds(line), ["d"], "{line}");
        assert_eq!(reached(line), Vec::<String>::new(), "{line}");
    }
}

#[test]
fn a_subshell_or_pipe_does_not_move_the_next_command() {
    for line in [
        "(cd x)",
        "(cd x) && ls",
        "cd x | cat",
        "cat | cd x",
        "cd x &",
        "(cd x; cd y); ls",
    ] {
        assert_eq!(reached(line), Vec::<String>::new(), "{line}");
    }
    assert_eq!(reached("cd x && ls; cd y"), ["x", "y"]);
    assert_eq!(
        reached("make 2>&1; cd x"),
        ["x"],
        "a redirect is no boundary"
    );
    assert_eq!(reached("cd x 2>&1 && ls"), ["x"]);
}

#[test]
fn payloads_are_read_to_depth_four() {
    assert_eq!(cds("bash -c 'cd x'"), ["x"]);
    assert_eq!(cds("sh -lc 'cd x'"), ["x"]);
    assert_eq!(cds("bash -o pipefail -c 'cd x'"), ["x"]);
    assert_eq!(cds("bash -eo pipefail -c 'cd x'"), ["x"]);
    assert_eq!(cds("bash -ceo pipefail 'cd x'"), ["x"]);
    assert_eq!(cds("bash --rcfile x -c 'cd y'"), ["y"]);
    assert_eq!(cds("bash --init-file x -c 'cd y'"), ["y"]);
    assert_eq!(cds("bash --noprofile -c 'cd y'"), ["y"]);
    assert_eq!(cds("eval 'cd x'"), ["x"]);
    assert_eq!(reached("eval 'cd x'"), ["x"], "eval's cwd persists");
    assert_eq!(reached("bash -c 'cd x'"), Vec::<String>::new());
    let nest = |n: usize| {
        let mut line = "cd x".to_string();
        for _ in 0..n {
            line = format!("eval {}", shell_quote(&line));
        }
        line
    };
    assert_eq!(cds(&nest(4)), ["x"]);
    assert!(cds(&nest(5)).is_empty(), "past the bound nothing is read");
    // A script file is a word, not a payload.
    assert!(cds("bash cd.sh").is_empty());
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

#[test]
fn command_v_is_a_probe() {
    assert!(events("command -v bash").expect("readable").is_empty());
    assert!(events("command -V cd").expect("readable").is_empty());
}

#[test]
fn a_flag_after_double_dash_is_the_scripts() {
    let flags = |line: &str| -> Vec<(String, String)> {
        events(line)
            .expect("readable")
            .into_iter()
            .filter_map(|event| match event {
                Event::Flag { flag, target } => Some((flag, target)),
                _ => None,
            })
            .collect()
    };
    let one = |flag: &str, target: &str| vec![(flag.to_string(), target.to_string())];
    assert!(flags("npm run package -- --prefix=../main/").is_empty());
    assert_eq!(
        flags("npm --prefix ../main test"),
        one("--prefix", "../main")
    );
    assert_eq!(flags("pnpm --dir=../main test"), one("--dir", "../main"));
    assert_eq!(flags("yarn --cwd ../main test"), one("--cwd", "../main"));
    assert_eq!(flags("make -C ../main"), one("-C", "../main"));
    assert_eq!(flags("make -C../main"), one("-C", "../main"));
    assert_eq!(flags("make -wC ../main"), one("-C", "../main"));
    assert_eq!(flags("make -wC../main"), one("-C", "../main"));
    assert_eq!(flags("make -f x -C ../main"), one("-C", "../main"));
    assert!(flags("make -fC x").is_empty(), "-f takes the C");
    assert_eq!(flags("pnpm -rC ../main test"), one("-C", "../main"));
    assert_eq!(
        flags("make --directory=../main"),
        one("--directory", "../main")
    );
    assert_eq!(flags("git -c a=b -C ../main log"), one("-C", "../main"));
    assert!(
        flags("git log -C ../main").is_empty(),
        "git's -C comes before its subcommand"
    );
}

#[test]
fn the_sentinel_is_exempt_for_its_own_command_only() {
    assert!(missing_sentinel("MISSING"));
    assert!(missing_sentinel("MISSING - Wave 0, can't yet"));
    assert!(!missing_sentinel("MISSINGNO"));
    assert!(!missing_sentinel("MISSING ; cd x"));
    assert!(!missing_sentinel("MISSING x\ncd y"));
    assert!(events("MISSING - Wave 0").expect("readable").is_empty());
    assert_eq!(cds("MISSING ; cd x"), ["x"]);
    assert_eq!(cds("MISSING x && cd y"), ["y"]);
}

#[test]
fn a_substitution_and_a_function_body_are_read_where_they_stand() {
    assert_eq!(cds(r#"echo "$(cd x && make)""#), ["x"]);
    assert_eq!(cds("echo `cd x`"), ["x"]);
    assert_eq!(cds("`cd x && make`"), ["x"]);
    assert_eq!(cds("function f { cd x; }; f"), ["x"]);
    assert_eq!(cds("function f() { cd x; }; f"), ["x"]);
    assert_eq!(cds("f() { cd x; }; f"), ["x"]);
    assert_eq!(reached(r#"echo "$(cd x)""#), Vec::<String>::new());
    assert_eq!(reached("echo `cd x`"), Vec::<String>::new());
    assert!(
        cds("echo '$(cd x)'").is_empty(),
        "the shell leaves it as text"
    );
}

#[test]
fn a_literal_prefix_ends_before_the_first_expansion() {
    assert_eq!(literal_prefix("/main/$X"), "/main/");
    assert_eq!(literal_prefix("/main/*"), "/main/");
    assert_eq!(literal_prefix("../main/a*"), "../main/");
    assert_eq!(literal_prefix("a*"), "");
    assert_eq!(literal_prefix("$X/y"), "");
    assert_eq!(literal_prefix("/main/x"), "/main/x");
}

#[test]
fn dynamic_targets_pass() {
    for target in ["$VAR/x", "`pwd`", "a*", "a?", "~/x"] {
        assert!(dynamic(target), "{target}");
    }
    assert!(!dynamic("../main"));
    assert_eq!(cds("cd"), ["~"]);
    assert_eq!(cds("cd -"), ["$OLDPWD"]);
    assert_eq!(cds("cd -P -- x"), ["x"]);
    assert_eq!(cds("cd -LP x"), ["x"]);
    assert_eq!(cds("cd -LP -- x"), ["x"]);
    assert_eq!(cds("pushd -n x"), ["x"]);
}

#[test]
fn absolute_words_are_found_where_a_path_can_sit() {
    let paths = |line: &str| -> Vec<String> {
        events(line)
            .expect("readable")
            .into_iter()
            .filter_map(|event| match event {
                Event::Absolute(path) => Some(path),
                _ => None,
            })
            .collect()
    };
    assert_eq!(paths("cat /a/b"), ["/a/b"]);
    assert_eq!(paths("FOO=/a make"), ["/a"]);
    assert_eq!(paths("tool --out=/a x"), ["/a"]);
    assert_eq!(paths("make >/a 2>/b"), ["/a", "/b"]);
    assert_eq!(paths("make&>/a"), ["/a"]);
    assert_eq!(paths(">/a"), ["/a"]);
    assert_eq!(paths("PATH=/a/bin:$PATH make"), ["/a/bin"]);
    assert_eq!(paths("PATH=$PATH:/a/bin make"), ["/a/bin"]);
    assert_eq!(paths("x=y:/a"), ["/a"]);
    assert_eq!(paths(r#"echo "$(cat /a)""#), ["/a"]);
    assert_eq!(paths("echo `cat /a`"), ["/a"]);
    assert_eq!(
        paths("python3 -c 'import os; os.chdir(\"/a/b\"); p = a/b'"),
        ["/a/b"]
    );
    assert_eq!(paths("node -e 'x(\"/c\")' /d"), ["/c", "/d"]);
    assert!(paths("echo a/b").is_empty());
}

#[test]
fn an_unclosed_quote_is_refused() {
    assert!(events("echo 'a").is_err());
    assert!(events("bash -c 'echo \"a'").is_err(), "in a payload too");
}
