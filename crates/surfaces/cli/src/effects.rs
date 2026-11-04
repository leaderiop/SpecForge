//! Which effects mode a command's compile runs in (ADR 0337 D4): `--effects` when given; else `check`, and `collect`
//! when a selected collector reads the checks' outcomes (ADR 0336), run `auto` when someone can answer (human output, a
//! terminal at stdin) and it is not CI, and every other command, and every unattended run, runs `cached`, performing
//! no effect.

use specforge_effects::EffectsMode;

/// `--effects <mode>`: on `check` and `watch`, the commands that run the checks to report them, and on `collect`, whose
/// collector may read their outcomes.
#[derive(clap::Args, Debug, Clone, Copy, Default)]
pub(crate) struct EffectsOption {
    /// Whether the run performs, reuses or replays an http or process effect (ADR 0337): `check` and a `collect` whose
    /// collector reads the checks' outcomes run `auto` when someone can answer outside CI, every other run `cached`.
    #[arg(long, value_enum)]
    pub(crate) effects: Option<EffectsMode>,
}

/// The command a compile is for: only `check` and `collect` may default to performing effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Command {
    Check,
    Collect,
    Other,
}

/// The mode of a compile for `command`, `asked` with `--effects`, when someone `can_answer`
/// (`crate::asking::can_answer`) and the run is or is not `ci`.
pub(crate) fn mode(
    asked: Option<EffectsMode>,
    command: Command,
    can_answer: bool,
    ci: bool,
) -> EffectsMode {
    match asked {
        Some(asked) => asked,
        None if command != Command::Other && can_answer && !ci => EffectsMode::Auto,
        None => EffectsMode::Cached,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asking::can_answer;
    use crate::outcome::OutputFormat;
    use crate::terminal::Facts;

    const TERMINAL: Facts = Facts {
        interactive: true,
        ..Facts::PIPED
    };

    #[test]
    fn a_terminal_check_is_auto() {
        let human = can_answer(OutputFormat::Human, TERMINAL);
        assert_eq!(mode(None, Command::Check, human, false), EffectsMode::Auto);
        assert_eq!(
            mode(None, Command::Collect, human, false),
            EffectsMode::Auto
        );
    }

    #[test]
    fn nobody_to_ask_or_ci_is_cached() {
        // JSON output at a terminal: nobody answers.
        let json = can_answer(OutputFormat::Json, TERMINAL);
        assert_eq!(mode(None, Command::Check, json, false), EffectsMode::Cached);
        // No terminal at stdin.
        let piped = can_answer(OutputFormat::Human, Facts::PIPED);
        assert_eq!(
            mode(None, Command::Check, piped, false),
            EffectsMode::Cached
        );
        assert_eq!(mode(None, Command::Check, true, true), EffectsMode::Cached);
    }

    #[test]
    fn every_other_command_is_cached() {
        assert_eq!(mode(None, Command::Other, true, false), EffectsMode::Cached);
    }

    #[test]
    fn effects_always_wins() {
        for asked in EffectsMode::ALL {
            assert_eq!(mode(Some(asked), Command::Check, false, true), asked);
            assert_eq!(mode(Some(asked), Command::Other, true, false), asked);
        }
    }
}
