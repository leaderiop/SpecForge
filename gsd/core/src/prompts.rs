//! The `spec_phase` prompt: the judge of `gsd/E202` and `gsd/W203` (`judged.rs`, D-r40-11). It ports no workflow
//! text: it tells a model how to read the two rules' inputs and how to record its verdict. Round r42 ports GSD's
//! spec-phase workflow into this id.

use specforge_extension_sdk::prelude::*;

const THRESHOLDS: &str =
    "goal >= 75, boundary >= 70, constraint >= 65, acceptance >= 70, ambiguity <= 20";

fn text(spec: &str) -> String {
    format!(
        "Judge the phase spec `{spec}`.\n\n\
         Two rules need your verdict. Read the inputs of each with `specforge query <entity> --inputs gsd/<code> \
         --format json`, judge, then record the outcome with `specforge gsd record-verdict <entity> <code> <outcome> \
         <model> '<basis>'`, where `<basis>` is the `basis` object that query printed.\n\n\
         - `gsd/E202` on `{spec}`: does the spec meet its ambiguity gates? Score it in percent ({THRESHOLDS}). \
         Record `passed` or `failed` and pass the five scores as `--values '{{\"scores\": {{\"goal\": 0, \"boundary\": 0, \
         \"constraint\": 0, \"acceptance\": 0, \"ambiguity\": 0}}}}'` (your own numbers).\n\
         - `gsd/W203` on each locked requirement of the spec (`{spec}/LR1`, ...): is its acceptance falsifiable, \
         something a test can fail? Record `passed` or `failed`.\n\n\
         Judge what the inputs show, not what the spec hopes. Name each problem in `--findings`."
    )
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.prompt("spec_phase", |p| {
        p.title("Judge a phase spec")
            .description("Judge a phase spec's ambiguity gates (gsd/E202) and its locked requirements' acceptance (gsd/W203)")
            .arg("spec", |a| {
                a.reference("phase_spec").required();
            })
            .subject("spec")
            .handler(|call| Ok(PromptOutput::user(text(call.str("spec").unwrap_or_default()))));
    });
}
