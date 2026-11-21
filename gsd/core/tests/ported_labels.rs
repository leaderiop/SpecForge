//! The phase labels the lifecycle prompts print: where each prompt reads a phase's number and name.

use gsd_core::prompts::{asset, render_text};

#[path = "common/prompt_combos.rs"]
mod prompt_combos;

use prompt_combos::prompts;

/// A phase's label is `Phase <its id's last segment>: <its name>`: plan-phase's step 3 and autonomous's phase details
/// say which part of the node each value is (a node holds no `phase_number` key), `{X}` is defined, and plan-phase's
/// own body prints no `{Name}` that no step sets.
#[test]
fn the_phase_label_is_read_from_the_id_and_the_name_field() {
    let all = prompts();
    let rendered = |command: &str| {
        let (_, combos) = all.iter().find(|(c, _)| c == command).expect("the prompt");
        render_text(command, "claude-code", &combos[0].0, &combos[0].1).expect("a render")
    };
    let plan = rendered("plan-phase");
    let (_, step) = plan.split_once("## 3. Validate Phase").expect("step 3");
    let step = step.split_once("\n## ").expect("step 4").0;
    let autonomous = rendered("autonomous");
    for (text, said) in [
        (step, "`phase_number` is the last segment of its `id`"),
        (step, "`phase_name` is its `.fields.name`"),
        (step, "`{X}` is `phase_number`"),
        (
            step,
            "a phase's label reads \"Phase {phase_number}: {phase_name}\"",
        ),
        (
            &autonomous,
            "`PHASE_NAME` (`phase_name`) is its `.fields.name`",
        ),
        (&autonomous, "`PHASE_NUM` is the last segment of its `id`"),
        (
            &autonomous,
            "a phase's label reads \"Phase ${PHASE_NUM}: ${PHASE_NAME}\"",
        ),
    ] {
        assert!(text.contains(said), "{said:?} is not said");
    }
    let own = asset("gsd-core/workflows/plan-phase.md").expect("an asset");
    assert!(!own.contains("{Name}"), "plan-phase prints a `{{Name}}`");
}
