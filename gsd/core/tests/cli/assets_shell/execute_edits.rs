//! The execute family's shell lines, run as the assets hold them on a taskboard copy under bash with only `specforge`
//! on the `PATH`.

use super::{asset, block};

/// The `<required_reading>` block of execute-phase's executor spawn: the spawn's prompt sits in a `text` fence, so the
/// block is read from the dispatch marker on, not as an HTML block.
fn executor_reading(text: &str) -> String {
    let at = text
        .find("[gsd:dispatch phase=")
        .expect("the executor spawn's dispatch marker");
    block(&text[at..], "<required_reading>", "</required_reading>")
}

/// The executor spawn hands the executor `{phase_dir}/{plan_file}` as its plan, and no fence of the workflow assigns
/// either: the executor's reading list carries no plan.
#[test]
fn the_executor_spawn_reads_paths_no_step_sets() {
    let text = asset("gsd-core/workflows/execute-phase.md");
    let reading = executor_reading(&text);
    assert!(reading.contains("{phase_dir}/{plan_file}"), "{reading}");
    let assigns = regex::Regex::new(r"(?m)\b(phase_dir|plan_file)=").expect("a regex");
    for fence in gsd_kit::markdown::fences(&text) {
        assert!(
            !assigns.is_match(&fence.text),
            "a fence sets it:\n{}",
            fence.text
        );
    }
}
