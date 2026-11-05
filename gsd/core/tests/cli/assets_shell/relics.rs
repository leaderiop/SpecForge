//! Upstream's state, read here as the graph's: the UI gate reads a computed field, and closing a milestone writes no
//! roadmap file (ADR 0343 D3).

use super::{Shell, asset, block, q};

/// The UI gate reads the phase's computed `frontend` field (`@gsd/ui` computes it): no field holds upstream's
/// `**UI hint**: yes`, so a grep of the phase's text for it is always false.
#[test]
#[ignore = "pin: the UI gate still greps the phase's text for a UI hint"]
fn the_ui_gate_reads_the_phases_frontend_field() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/progress.md");
    let lines: Vec<&str> = text.lines().map(str::trim_start).collect();
    let at = lines
        .iter()
        .position(|l| l.starts_with("PHASE_SECTION=") || l.starts_with("PHASE_HAS_UI="))
        .expect("the UI gate");
    let gate = lines[at..at + 2].join("\n");
    for (answer, want) in [("true", "true"), ("", "false")] {
        let stub = format!(
            "specforge() {{ case \"$*\" in *fields.frontend*) printf '%s' {a};; *) [ -n {a} ] && printf '## Phase 1\\n';; esac; }}",
            a = q(answer)
        );
        let out = sh.run(&format!(
            "{stub}\nCURRENT_PHASE=v1.0/01\n{gate}\nprintf '%s' \"$PHASE_HAS_UI\""
        ));
        assert_eq!(out, want, "{answer:?}");
    }
}

/// Closing a milestone writes no roadmap file: the roadmap is the `milestone` and `phase` entities. Upstream's sentinel
/// line `printf … > <path>`, with its path swapped for prose, wrote a file named `a`.
#[test]
#[ignore = "pin: the milestone close still writes a scratch file"]
fn the_milestone_close_writes_no_scratch_file() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/complete-milestone.md");
    let step = block(&text, "**Reorganize ROADMAP.md**", "**Safety commit");
    let mut code = Vec::new();
    let mut fenced = false;
    for l in step.lines() {
        if l.trim_start().starts_with("```") {
            fenced = !fenced && l.trim() == "```bash";
            continue;
        }
        if fenced {
            code.push(l);
        }
    }
    let tmp = tempfile::tempdir().expect("a dir");
    let (_, err, _) = sh.run_in(
        tmp.path(),
        &format!("specforge() {{ :; }}\n{}", code.join("\n")),
    );
    let left: Vec<_> = std::fs::read_dir(tmp.path())
        .expect("the dir")
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert!(left.is_empty(), "{left:?}: {err}");
}
