//! `specforge files search` on ripgrep's crates (ADR 0339's amendment): context, multiline and inverted matches,
//! `--and-not`, the file and count modes, a missing path, and the `-q` and `--none` verdicts.

use super::project;
use crate::support::{Cli, TestProject};

/// A line with two matches is answered once, at its first; a CRLF line's text holds no `\r`.
#[test]
fn search_reports_every_line_once_and_the_first_column() {
    let project = project().files(&[("notes/crlf.txt", "say hello, hello\r\nno\r\nhello\r\n")]);
    let run = Cli::on(&project)
        .run(&["files", "search", "hello", "notes"])
        .exits(0)
        .json();
    let matches = run["matches"].as_array().cloned().unwrap_or_default();
    let seen: Vec<(u64, u64, &str)> = matches
        .iter()
        .map(|found| {
            (
                found["line"].as_u64().unwrap_or(0),
                found["column"].as_u64().unwrap_or(0),
                found["text"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(seen, [(1, 5, "say hello, hello"), (3, 1, "hello")], "{run}");
    // A line anchor holds at a CRLF line's end.
    let anchored = Cli::on(&project)
        .run(&["files", "search", "hello$", "notes", "--pick", ".count"])
        .exits(0);
    assert_eq!(anchored.stdout, "2\n");
}

/// A project with `notes/ctx.txt`: `hit` on lines 2, 5 and 10.
fn context_project() -> TestProject {
    project().files(&[(
        "notes/ctx.txt",
        "a\nhit one\nb\nc\nhit two\nd\ne\nf\ng\nhit three\n",
    )])
}

/// The library reports each line once: a line between two close matches is the earlier match's `after`, never the
/// later's `before`.
#[test]
fn context_lines_belong_to_the_nearest_match() {
    let project = context_project();
    let run = Cli::on(&project)
        .run(&["files", "search", "hit", "notes", "-C", "2"])
        .exits(0);
    insta::assert_json_snapshot!("files_search_context", run.answer());
    let human = Cli::on(&project)
        .run(&[
            "files", "search", "hit", "notes", "-B", "1", "-A", "0", "-C", "3", "--format", "human",
        ])
        .exits(0);
    assert_eq!(
        human.stdout,
        "notes/ctx.txt-1- a\nnotes/ctx.txt:2:1: hit one\nnotes/ctx.txt-4- c\nnotes/ctx.txt:5:1: hit two\n\
         notes/ctx.txt-9- g\nnotes/ctx.txt:10:1: hit three\n"
    );
}

/// `-U 'X(.*\n){0,N}?.*Y'`: a Y line within N lines after an X line, Y on X's own line included.
#[test]
fn a_multiline_pattern_finds_y_within_n_lines_of_x() {
    let project = project().files(&[("notes/xy.txt", "X start\na\nb\nY end\n-\nX and Y\n")]);
    let count = |n: usize| {
        let pattern = format!("X(.*\\n){{0,{n}}}?.*Y");
        Cli::on(&project)
            .run(&[
                "files",
                "search",
                "-U",
                &pattern,
                "notes",
                "--pick",
                ".matches[].line",
            ])
            .exits(0)
            .stdout
    };
    // The library joins matches on adjacent lines into one, so the fixture keeps them apart.
    assert_eq!(count(3), "1\n6\n");
    assert_eq!(count(2), "6\n");
    let spans = Cli::on(&project)
        .run(&[
            "files",
            "search",
            "-U",
            "X(.*\\n){0,3}?.*Y",
            "notes",
            "--pick",
            ".matches[0]",
        ])
        .exits(0)
        .json();
    assert_eq!(spans["end_line"], 4, "{spans}");
    assert_eq!(spans["text"], "X start\na\nb\nY end", "{spans}");
}

#[test]
fn a_newline_pattern_without_u_is_refused() {
    let project = project();
    let run = Cli::on(&project)
        .run(&["files", "search", "a\\nb", "src"])
        .exits(1);
    assert_eq!(run.json()["code"], "invalid_input", "{}", run.stdout);
    assert!(
        run.json()["error"]
            .as_str()
            .unwrap_or_default()
            .contains("not allowed"),
        "{}",
        run.stdout
    );
}

#[test]
fn an_inverted_search_answers_the_other_lines() {
    let project = project();
    let run = Cli::on(&project)
        .run(&[
            "files",
            "search",
            "-v",
            "hello",
            "src/main.rs",
            "--pick",
            ".matches[]",
        ])
        .exits(0);
    assert_eq!(
        run.stdout,
        "{\"column\":1,\"line\":1,\"path\":\"src/main.rs\",\"text\":\"fn main() {\"}\n\
         {\"column\":1,\"line\":3,\"path\":\"src/main.rs\",\"text\":\"}\"}\n"
    );
}

#[test]
fn fixed_strings_match_a_regex_character_literally() {
    let project = project().files(&[("notes/dots.txt", "a.b\naxb\n")]);
    let regex = Cli::on(&project)
        .run(&["files", "search", "a.b", "notes", "--pick", ".count"])
        .exits(0);
    assert_eq!(regex.stdout, "2\n");
    let fixed = Cli::on(&project)
        .run(&[
            "files",
            "search",
            "-F",
            "a.b",
            "notes",
            "--pick",
            ".matches[].line",
        ])
        .exits(0);
    assert_eq!(fixed.stdout, "1\n");
}

/// `--and-not` drops a reported line its pattern matches, under `-i` and `-F` as the search's own pattern, against a
/// `-U` match whole; the context lines stay.
#[test]
fn and_not_leaves_out_the_lines_it_matches() {
    let project = project().files(&[(
        "notes/uses.ts",
        "import { useAuth } from './auth'\nconst a = useAuth()\n// useAuth.x here\nIMPORT useAuth\n",
    )]);
    let lines = |args: &[&str]| {
        let mut all = vec!["files", "search"];
        all.extend_from_slice(args);
        all.extend_from_slice(&["notes", "--pick", ".matches[].line"]);
        Cli::on(&project).run(&all).exits(0).stdout
    };
    assert_eq!(lines(&["useAuth", "--and-not", "import"]), "2\n3\n4\n");
    assert_eq!(lines(&["-i", "useAuth", "--and-not", "import"]), "2\n3\n");
    assert_eq!(
        lines(&["-F", "useAuth", "--and-not", "useAuth.x"]),
        "1\n2\n4\n"
    );
    assert_eq!(
        lines(&["useAuth", "--and-not", "import", "--and-not", "^//"]),
        "2\n4\n"
    );
    // Against a multiline match whole: the match of lines 1-2 holds `import`.
    assert_eq!(
        lines(&["-U", "useAuth.*\\n.*useAuth\\(", "--and-not", "import"]),
        ""
    );
    // A kept match keeps its context, a line the second pattern matches included.
    let context = Cli::on(&project)
        .run(&[
            "files",
            "search",
            "const",
            "notes",
            "-B",
            "1",
            "--and-not",
            "x",
            "--pick",
            ".matches[0].before[].line",
        ])
        .exits(0);
    assert_eq!(context.stdout, "1\n");
}

/// `-l`: each file once, its search stopped at its first match; `--max` bounds the files.
#[test]
fn files_with_matches_stops_at_each_files_first_match() {
    let project = project().files(&[("src/twice.rs", "hello\nhello\n")]);
    let run = Cli::on(&project)
        .run(&["files", "search", "-l", "hello", "src", "build/out.txt"])
        .exits(0);
    insta::assert_json_snapshot!("files_search_files", run.answer());
    let first = Cli::on(&project)
        .run(&[
            "files", "search", "-l", "hello", "src", "--max", "1", "--format", "human",
        ])
        .exits(0);
    assert_eq!(first.stdout, "src/lib/util.rs\n(stopped at 1 answered)\n");
}

/// `--counts`: every file with a match, and a file typed by name even at 0, as `grep -c` names it.
#[test]
fn counts_name_every_typed_file_even_at_zero() {
    let project = project().files(&[("src/twice.rs", "hello\nhello\n")]);
    let run = Cli::on(&project)
        .run(&["files", "search", "--counts", "hello", "spec/a.spec", "src"])
        .exits(0);
    insta::assert_json_snapshot!("files_search_counts", run.answer());
    let human = Cli::on(&project)
        .run(&[
            "files",
            "search",
            "--counts",
            "hello",
            "src/twice.rs",
            "--format",
            "human",
        ])
        .exits(0);
    assert_eq!(human.stdout, "src/twice.rs:2\n");
}

#[test]
fn list_and_count_modes_refuse_context() {
    let project = project();
    for mode in ["-l", "--counts"] {
        Cli::on(&project)
            .run(&["files", "search", mode, "hello", "-C", "1"])
            .exits(2);
    }
    Cli::on(&project)
        .run(&["files", "search", "-l", "--counts", "hello"])
        .exits(2);
}

#[test]
fn a_missing_path_is_listed_and_the_rest_searched() {
    let project = project();
    let run = Cli::on(&project)
        .run(&["files", "search", "hello", "nosuch.ts", "src"])
        .exits(0)
        .json();
    assert_eq!(run["missing"], serde_json::json!(["nosuch.ts"]), "{run}");
    assert_eq!(run["count"], 2, "{run}");
    let human = Cli::on(&project)
        .run(&[
            "files",
            "search",
            "hello",
            "nosuch.ts",
            "src/lib",
            "--format",
            "human",
        ])
        .exits(0);
    assert_eq!(
        human.stdout,
        "src/lib/util.rs:1:8: pub fn hello() {}\n(missing: nosuch.ts)\n"
    );
}

/// The exit of `files search <flag> <args>`, and that it printed nothing on stdout.
fn verdict(project: &TestProject, flag: &str, args: &[&str]) -> i32 {
    let mut all = vec!["files", "search", flag];
    all.extend_from_slice(args);
    let run = Cli::on(project).run(&all);
    assert_eq!(run.stdout, "", "{all:?}");
    run.code
}

/// A project with a file the host withholds from every read.
fn project_with_a_secret() -> TestProject {
    project().files(&[(".env", "hello=1\n")])
}

#[test]
fn quiet_exits_0_on_a_match_1_on_none_2_when_unjudged() {
    let project = project_with_a_secret();
    assert_eq!(verdict(&project, "-q", &["hello", "src"]), 0);
    assert_eq!(verdict(&project, "-q", &["zzz", "src"]), 1);
    assert_eq!(verdict(&project, "-q", &["zzz", "nosuch.ts", "src"]), 2);
    assert_eq!(verdict(&project, "-q", &["hello", "nosuch.ts", "src"]), 0);
    // A file typed by name that is never read is no verdict: refused on stderr.
    assert_eq!(verdict(&project, "-q", &["hello", ".env"]), 2);
    let run = Cli::on(&project).run(&["files", "search", "-q", "hello", ".env"]);
    assert!(
        run.stderr.contains("error[permission_denied]"),
        "{}",
        run.stderr
    );
}

#[test]
fn none_exits_0_on_none_1_on_a_match_2_when_unjudged() {
    let project = project_with_a_secret();
    assert_eq!(verdict(&project, "--none", &["zzz", "src"]), 0);
    assert_eq!(verdict(&project, "--none", &["hello", "src"]), 1);
    assert_eq!(verdict(&project, "--none", &["zzz", "nosuch.ts"]), 2);
    assert_eq!(verdict(&project, "--none", &["zzz", ".env"]), 2);
    let outside = tempfile::tempdir().unwrap();
    let typed = outside.path().display().to_string();
    assert_eq!(verdict(&project, "--none", &["zzz", &typed]), 2);
}

#[test]
fn quiet_takes_no_pick() {
    let project = project();
    for args in [
        &["-q", "hello", "--pick", ".count"][..],
        &["--none", "hello", "--format", "human"],
        &["-q", "--none", "hello"],
        &["-q", "-l", "hello"],
        &["--none", "-C", "1", "hello"],
    ] {
        let mut all = vec!["files", "search"];
        all.extend_from_slice(args);
        Cli::on(&project).run(&all).exits(2);
    }
}

#[test]
fn a_path_outside_the_roots_still_refuses() {
    let project = project();
    let outside = tempfile::tempdir().unwrap();
    let typed = outside.path().display().to_string();
    let run = Cli::on(&project)
        .run(&["files", "search", "x", "src", &typed])
        .exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
}

/// `-` is standard input, named `<stdin>`, beside other paths; `-q` judges it.
#[test]
fn a_dash_searches_standard_input() {
    let project = project();
    let run = Cli::on(&project)
        .stdin("one\nvite@7.1.0\n")
        .run(&[
            "files",
            "search",
            "vite@7",
            "-",
            "src/main.rs",
            "--pick",
            ".matches[].path",
        ])
        .exits(0);
    assert_eq!(run.stdout, "<stdin>\n");
    let hello = Cli::on(&project)
        .stdin("x\n")
        .run(&[
            "files",
            "search",
            "hello",
            "-",
            "src/main.rs",
            "--pick",
            ".matches[].path",
        ])
        .exits(0);
    assert_eq!(hello.stdout, "src/main.rs\n");
    let quiet = |input: &str| {
        Cli::on(&project)
            .stdin(input)
            .run(&["files", "search", "-q", "[0-9]+ passed", "-"])
            .code
    };
    assert_eq!(quiet("12 passed\n"), 0);
    assert_eq!(quiet("failed\n"), 1);
}

/// With no `-`, a search never reads standard input: an open stdin does not hold it.
#[test]
fn no_dash_never_reads_stdin() {
    let project = project();
    let root = project.root().display().to_string();
    let mut child =
        Cli::on(&project).start(&["files", "search", "-q", "hello", "src", "--path", &root]);
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(60) {
            let _ = child.kill();
            panic!("the search waited on its open stdin");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    // The stdin pipe is still open here: the child ended without it.
    drop(child.stdin.take());
    assert_eq!(status.code(), Some(0));
}
