//! What an extension's text may write to the terminal (ADR 0244).

use crate::support::registry::greet_with;
use crate::support::{Cli, TestProject};
use specforge_test_macros::test as specforge_test;

#[specforge_test(
    invariant = "wasm_sandbox_integrity",
    verify = "no text an extension supplies reaches the terminal as a control sequence"
)]
fn an_extensions_text_reaches_the_terminal_printable() {
    // Same-length patches of greet's help text and of its rule and pass message text.
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&["@specforge/software"])
        .file("main.spec", "greeting hello \"Hello\" {\n  style loud\n}\n");
    project.write(
        "greet.wasm",
        greet_with(&[
            ("Greet someone, warmly", "\u{1b}]52;c;cm0gLXJmIH4=\u{7}X"),
            ("greeting '", "\u{1b}]0;PWNED\u{7}"),
        ]),
    );
    Cli::on(&project)
        .command(&["add", "greet.wasm"])
        .assert()
        .success();

    let colored = |args: &[&str]| {
        let run = Cli::on(&project).env("CLICOLOR_FORCE", "1").run(args);
        format!("{}{}", run.stdout, run.stderr)
    };
    for args in [&["greet", "hello", "--help"][..], &["check"], &["analyze"]] {
        let text = colored(args);
        assert!(!text.contains("\u{1b}]"), "{args:?}: {text:?}");
        assert!(text.contains("\\u{1b}]"), "{args:?}: {text:?}");
    }
}
