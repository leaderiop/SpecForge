//! The `--pick` shapes on an extension command (ADR 0325 D8): host options beside `--pick`, each unless the command
//! declares an arg of the name.

use super::*;
use crate::pick::Picked;

#[test]
fn the_shapes_print_a_commands_answer() {
    let answer = r#"{"a": [1, 2, 3]}"#;
    for (shape, printed) in [
        (&["--count"][..], "3\n"),
        (&["--first", "2"], "1\n2\n"),
        (&["--last", "1", "--join", ","], "3\n"),
        (&["--join", ", "], "1, 2, 3\n"),
        (&["--join", "-x"], "1-x2-x3\n"),
    ] {
        let mut args = vec!["--pick", ".a[]"];
        args.extend(shape);
        let (code, out, err) = quiet(answer, "", 0, &args);
        assert_eq!((code, out.as_str()), (0, printed), "{shape:?}: {err}");
    }
    // the exit code is the command's
    let (code, out, _) = quiet(answer, "", 3, &["--pick", ".a[]", "--count"]);
    assert_eq!((code, out.as_str()), (3, "3\n"));
}

#[test]
fn a_shape_without_pick_is_a_usage_error() {
    for args in [
        &["--count"][..],
        &["--first", "1"],
        &["--last", "1"],
        &["--join", ","],
    ] {
        let (code, out, err) = quiet(r#"{"a": [1]}"#, "", 0, args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}: {err}");
    }
}

#[test]
fn shapes_that_conflict_are_a_usage_error() {
    for args in [
        &["--pick", ".a[]", "--count", "--last", "1"][..],
        &["--pick", ".a[]", "--count", "--join", ","],
        &["--pick", ".a[]", "--first", "1", "--last", "1"],
    ] {
        let (code, out, err) = quiet(r#"{"a": [1]}"#, "", 0, args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}: {err}");
    }
}

#[test]
fn a_command_declaring_count_keeps_it_and_still_takes_first() {
    let mut c = contribution();
    c.args
        .push(arg("count", CommandArgType::Integer, false, None));
    let command = declared(c.clone());
    let added = kernel_shapes(&command);
    assert_eq!(added, ["first", "last", "join"]);
    let argv = [
        "specforge x",
        "milestone-completion",
        "m1",
        "--count",
        "7",
        "--pick",
        ".a[]",
        "--first",
        "1",
    ];
    let matches = command_line("x", &[&command])
        .try_get_matches_from(argv)
        .unwrap();
    let (_, sub) = matches.subcommand().unwrap();
    // the declared `count` is the command's arg, not a shape
    let (args, _) = sent_by(&c, &argv[1..]).unwrap();
    assert_eq!(args["count"], 7);
    assert_eq!(
        Picked::of_matches(sub, &added).printed(r#"{"a": [1, 2, 3]}"#),
        Some(Ok("1\n".into()))
    );
    // a command that declares `pick` has none of the host's: no shape either
    let mut own = contribution();
    own.args
        .push(arg("pick", CommandArgType::String, false, None));
    assert!(kernel_shapes(&declared(own)).is_empty());
}

#[test]
fn a_shape_conflicts_with_no_arg_the_command_declares() {
    let mut c = contribution();
    c.args
        .push(arg("join", CommandArgType::String, false, None));
    let command = declared(c.clone());
    assert_eq!(kernel_shapes(&command), ["count", "first", "last"]);
    // the host's `--count` takes no other shape, but the declared `--join` is the command's
    let argv = [
        "milestone-completion",
        "m1",
        "--join",
        "x",
        "--pick",
        ".a[]",
        "--count",
    ];
    let (args, _) = sent_by(&c, &argv).unwrap();
    assert_eq!(args["join"], "x");
}

#[test]
fn the_host_count_never_reaches_the_guests_arguments() {
    let c = contribution();
    let command = declared(c.clone());
    assert!(!command.args().iter().any(|a| a.name == "count"));
    let argv = ["milestone-completion", "m1", "--pick", ".a[]", "--count"];
    // the host reads it
    let matches = command_line("x", &[&command])
        .try_get_matches_from(std::iter::once("specforge x").chain(argv))
        .unwrap();
    let (_, sub) = matches.subcommand().unwrap();
    assert_eq!(
        Picked::of_matches(sub, &kernel_shapes(&command)).printed(r#"{"a": [1, 2]}"#),
        Some(Ok("2\n".into()))
    );
    // the guest is sent only what it declares
    let (args, _) = sent_by(&c, &argv).unwrap();
    for host in ["count", "pick"] {
        assert!(!args.contains_key(host), "{host}: {args:?}");
    }
    assert!(
        args.keys().all(|key| c.args.iter().any(|a| a.name == *key)),
        "{args:?}"
    );
}
