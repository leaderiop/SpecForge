//! The `--pick` shapes on an extension command (ADR 0325 D8): host options beside `--pick`, each unless the command
//! declares an arg of the name.

use super::*;
use crate::pick::Shape;

#[test]
fn the_shapes_print_a_commands_answer() {
    let answer = r#"{"a": [1, 2, 3]}"#;
    for (shape, printed) in [
        (&["--count"][..], "3\n"),
        (&["--first", "2"], "1\n2\n"),
        (&["--last", "1", "--join", ","], "3\n"),
        (&["--join", ", "], "1, 2, 3\n"),
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
        &["--pick", ".a[]", "--count", "--last", "1"],
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
    let shape = Shape::of_matches(sub, &added);
    let pick = Pick::parse(".a[]").unwrap();
    assert_eq!(
        pick.printed(r#"{"a": [1, 2, 3]}"#, None, &shape),
        Ok("1\n".into())
    );
    // a command that declares `pick` has none of the host's: no shape either
    let mut own = contribution();
    own.args
        .push(arg("pick", CommandArgType::String, false, None));
    assert!(kernel_shapes(&declared(own)).is_empty());
}
