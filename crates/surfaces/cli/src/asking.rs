//! Who a command can ask (ADR 0110): the terminal answers a publisher key change (`add`, `update`), a
//! collector's command (`collect`), and an extension's capability or a root outside the project (ADR 0335 D2.4)
//! when someone can answer.

use crate::outcome::OutputFormat;
use crate::terminal::{Facts, Terminal};
use specforge_ops::asking::{Asker, Asking, ExpectedFrom, Question};

/// Who a command can ask: `--yes` answers everything; a human-format command whose stdin is a terminal asks
/// it; under `--format json`, or with nobody at stdin, nobody can be asked.
pub(crate) fn of<'t>(yes: bool, format: OutputFormat, terminal: &'t Terminal) -> Asking<'t> {
    let answers = can_answer(format, terminal.facts());
    Asking::of(yes, answers.then_some(terminal as &dyn Asker))
}

/// Whether someone can answer a command of `format` run under `facts`: human output, and a terminal at stdin.
pub(crate) fn can_answer(format: OutputFormat, facts: Facts) -> bool {
    format == OutputFormat::Human && facts.interactive
}

impl Asker for Terminal {
    fn answer(&self, question: &Question<'_>) -> bool {
        match question {
            Question::KeyChange { .. } => self.ask(&key_change(question)),
            Question::Capability { .. } | Question::Root { .. } => self.ask(&approval(question)),
        }
    }
}

/// The key-change question as the terminal shows it (today's bytes).
fn key_change(question: &Question<'_>) -> String {
    let Question::KeyChange {
        package,
        expected,
        offered,
        from,
    } = question
    else {
        return String::new();
    };
    match from {
        ExpectedFrom::UserPin => format!(
            "KEY CHANGE for '{package}': pinned '{expected}' but new package is signed '{offered}'\ntrust the new key and re-pin? [y/N] "
        ),
        ExpectedFrom::Lock => format!(
            "KEY CHANGE for '{package}': specforge.lock records '{expected}' but new package is signed '{offered}'\ntrust the new key and pin it? [y/N] "
        ),
    }
}

/// A capability or outside-root question as the terminal shows it (ADR 0335 D2.4). A collector's command keeps
/// the bytes it had before capabilities: the extension, the project and the command.
fn approval(question: &Question<'_>) -> String {
    match question {
        Question::Capability {
            extension,
            project,
            command: Some(argv),
            ..
        } => format!(
            "{extension} wants to run this command in {project}:\n  {}\nAllow it for this project? [y/N] ",
            argv.join(" ")
        ),
        Question::Capability {
            extension,
            scope,
            summary,
            why,
            project,
            command: None,
        } => format!(
            "{extension} asks for '{scope}': {summary}\n  why: {why}\nAllow it for {project}? [y/N] "
        ),
        Question::Root {
            name,
            target,
            project,
        } => format!(
            "root '{name}' lies outside the project, at {target}\nAllow extensions to read it for {project}? [y/N] "
        ),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::Facts;

    const INTERACTIVE: Facts = Facts {
        color_out: false,
        color_err: false,
        interactive: true,
        ci: false,
    };

    fn change(from: ExpectedFrom) -> Question<'static> {
        Question::KeyChange {
            package: "@x/a",
            expected: "k1",
            offered: "k2",
            from,
        }
    }

    #[specforge_test_macros::test(
        behavior = "pin_publisher_key",
        verify = "the CLI asks its terminal about a key change: yes at the prompt consents, and with nobody to ask nothing is asked"
    )]
    fn a_key_change_is_asked_of_the_terminal() {
        let (terminal, capture) = Terminal::captured("y\n", INTERACTIVE);
        assert!((&terminal as &dyn Asker).answer(&change(ExpectedFrom::UserPin)));
        assert_eq!(
            capture.stderr(),
            "KEY CHANGE for '@x/a': pinned 'k1' but new package is signed 'k2'\ntrust the new key and re-pin? [y/N] "
        );

        let (terminal, capture) = Terminal::captured("y\n", INTERACTIVE);
        assert!((&terminal as &dyn Asker).answer(&change(ExpectedFrom::Lock)));
        assert_eq!(
            capture.stderr(),
            "KEY CHANGE for '@x/a': specforge.lock records 'k1' but new package is signed 'k2'\ntrust the new key and pin it? [y/N] "
        );

        let (terminal, _) = Terminal::captured("n\n", INTERACTIVE);
        assert!(!(&terminal as &dyn Asker).answer(&change(ExpectedFrom::UserPin)));

        let (terminal, capture) = Terminal::captured("y\n", Facts::PIPED);
        assert!(!(&terminal as &dyn Asker).answer(&change(ExpectedFrom::UserPin)));
        assert_eq!(capture.stderr(), "", "nobody to ask: nothing is asked");
    }

    #[specforge_test_macros::test(
        behavior = "approve_capability",
        verify = "a terminal that can answer is asked; with nobody to ask, or under --format json, only an approved command runs"
    )]
    fn a_collector_command_is_asked_of_a_terminal_that_can_answer() {
        let argv = vec!["cargo".to_string(), "test".to_string()];
        let question = Question::Capability {
            extension: "@specforge/cargo-test",
            scope: "cargo-test",
            summary: "process cargo test",
            why: "run the tests",
            project: "/p",
            command: Some(&argv),
        };
        let (terminal, capture) = Terminal::captured("y\n", INTERACTIVE);
        assert!(of(false, OutputFormat::Human, &terminal).answers(&question));
        assert_eq!(
            capture.stderr(),
            "@specforge/cargo-test wants to run this command in /p:\n  cargo test\nAllow it for this project? [y/N] "
        );

        let who = |yes, format, facts| {
            let (terminal, _) = Terminal::captured("", facts);
            format!("{:?}", of(yes, format, &terminal))
        };
        assert_eq!(who(false, OutputFormat::Json, INTERACTIVE), "Nobody");
        assert_eq!(who(false, OutputFormat::Human, Facts::PIPED), "Nobody");
        for format in [OutputFormat::Human, OutputFormat::Json] {
            assert_eq!(who(true, format, Facts::PIPED), "Yes");
        }
    }

    #[test]
    fn who_is_asked_is_yes_then_a_terminal_that_can_answer() {
        let who = |yes, format, facts| {
            let (terminal, _) = Terminal::captured("", facts);
            format!("{:?}", of(yes, format, &terminal))
        };

        assert_eq!(who(true, OutputFormat::Human, Facts::PIPED), "Yes");
        assert_eq!(who(true, OutputFormat::Json, INTERACTIVE), "Yes");
        assert_eq!(who(false, OutputFormat::Json, INTERACTIVE), "Nobody");
        assert_eq!(who(false, OutputFormat::Human, Facts::PIPED), "Nobody");
        assert_eq!(who(false, OutputFormat::Human, INTERACTIVE), "Ask");
    }
}
