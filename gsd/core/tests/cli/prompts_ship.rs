//! `complete_milestone` and `ship` (T10): the prompts gated by their guards. E512 is proved here black-box.

use super::commands_support::{gsd, strict, twin};
use super::harness::taskboard;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

const PHASE: &str = "v1.0/01";

/// The twin with its phase 01 verified (E511 passes) and an open window found in it.
fn with_open_window() -> super::harness::Project {
    let p = taskboard();
    assert_eq!(p.verify_phase(PHASE), 0);
    let run = gsd(
        &p,
        "record-window",
        &["todo", "01", "A TODO remains in the cli"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    p
}

#[test]
fn ship_gate_refuses_an_open_window() {
    let p = with_open_window();
    strict(&p);
    let before = p.read("specforge.json");
    let run = prompt(&p, "ship", &[PHASE], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("gate_failed"), "{}", run.all());
    assert!(run.all().contains("E512"), "{}", run.all());
    assert_eq!(p.read("specforge.json"), before);
    // After the window is fixed, it renders.
    let fixed = gsd(&p, "fix-window", &["BW1"]);
    assert_eq!(fixed.code, 0, "{}", fixed.all());
    let rendered = payload(&prompt(&p, "ship", &[PHASE], "claude-code"));
    assert_eq!(rendered["payload"]["subject"], PHASE);
}

#[test]
fn ship_renders_under_default_with_an_open_window() {
    // `workflow.windows_enforce` is off under the default preset: E512 is silent.
    let p = with_open_window();
    let rendered = payload(&prompt(&p, "ship", &[PHASE], "claude-code"));
    assert_eq!(rendered["payload"]["subject"], PHASE);
}

#[test]
fn ship_gate_refuses_an_unverified_phase() {
    let p = twin();
    let run = prompt(&p, "ship", &["v1.0/02"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E511"), "{}", run.all());
}

#[test]
fn complete_milestone_gate_refuses_without_a_passing_audit() {
    let p = twin();
    let run = prompt(&p, "complete_milestone", &["v1.0"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E106"), "{}", run.all());
    // The command refuses with the same codes.
    let command = gsd(&p, "close-milestone", &["v1.0"]);
    assert_ne!(command.code, 0, "{}", command.all());
    assert!(command.all().contains("E106"), "{}", command.all());
}

#[test]
fn complete_milestone_renders_for_three_hosts() {
    let p = super::cmd_milestone::ready();
    render_three_hosts(&p, "complete_milestone", &["v1.0"], "v1.0", "<objective>");
}

#[test]
fn ship_renders_for_three_hosts() {
    let p = taskboard();
    assert_eq!(p.verify_phase(PHASE), 0);
    render_three_hosts(&p, "ship", &[PHASE], "01", "<objective>");
}

#[test]
fn complete_milestone_names_close_milestone() {
    let p = super::cmd_milestone::ready();
    let rendered = payload(&prompt(&p, "complete_milestone", &["v1.0"], "claude-code"));
    assert_eq!(rendered["payload"]["steps"][0]["id"], "close");
    let title = rendered["payload"]["steps"][0]["title"]
        .as_str()
        .expect("a title");
    assert!(title.contains("specforge gsd close-milestone"), "{title}");
}

/// REVIEW-r42 N5: the ship text reads the milestone's ledger (every phase of it), as `contributions/README.md` says,
/// not one phase's windows and not a "project root" file.
#[test]
fn ship_reads_the_ledger_of_the_milestone() {
    let p = with_open_window();
    let text = text_of(&payload(&prompt(&p, "ship", &[PHASE], "claude-code")));
    assert!(
        text.contains("specforge query \"$MILESTONE\" --depth 2 --kinds broken_window"),
        "{text}"
    );
    assert!(
        !text.contains("project root"),
        "the ledger is the milestone's"
    );
}

/// #5195: PROJECT.md is shared across workstreams, so complete-milestone reviews it in flat mode only. The gate reads
/// the workstream `switch` made active (SpecForge's INIT has no `section_manifest`) and needs no `node`.
#[test]
fn complete_milestone_skips_the_project_review_under_an_active_workstream() {
    let p = super::harness::project("workstreams");
    let served = gsd(
        &p,
        "reference",
        &["gsd-core/workflows/complete-milestone.md"],
    );
    assert_eq!(served.code, 0, "{}", served.all());
    let gate = served
        .stdout
        .split("```bash\n")
        .filter_map(|b| b.split("\n```").next())
        .find(|b| b.contains("PROJECT_MD_REVIEW"))
        .expect("the review gate");
    let bin = std::path::PathBuf::from(std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"));
    let path = format!(
        "{}:{}",
        bin.parent().expect("a directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let review = || {
        let out = std::process::Command::new("bash")
            .args(["-c", gate])
            .env("PATH", &path)
            .current_dir(p.root())
            .output()
            .expect("bash");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("PROJECT_MD_REVIEW="))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(review(), "PROJECT_MD_REVIEW=run", "flat mode");
    let switched = gsd(&p, "workstreams", &["switch", "--name", "auth"]);
    assert_eq!(switched.code, 0, "{}", switched.all());
    assert_eq!(review(), "PROJECT_MD_REVIEW=skip", "under workstream auth");
    assert!(
        !served
            .stdout
            .lines()
            .any(|l| l.contains("node -e") && l.contains("section_manifest")),
        "a gate still reads `section_manifest` with node"
    );
}
