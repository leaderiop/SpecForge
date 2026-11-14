//! The discuss and project researchers' reads, run as the agents hold them under bash with only `specforge`: each
//! names its context pack, and the analyzer's search and anchors find the earlier phases' work.

use super::{Shell, asset, span};
use crate::harness::taskboard;

/// The command of the code span in agent `name` that starts with `prefix`, its placeholder (from the first `<` or
/// `"` on: `<phase>`, `"$PHASE_ID"`, `"<goal terms>"`) replaced by `subject`.
fn read_of(name: &str, prefix: &str, subject: &str) -> String {
    let text = asset(&format!("agents/{name}.md"));
    let span = span(&text, prefix);
    let at = span.find(['<', '"']).expect("a placeholder");
    format!("{}'{subject}'", &span[..at])
}

/// Every researcher's pack route, as its agent names it, compiles on the taskboard; the phase packs carry the earlier
/// phase's context, and the analyzer's search finds the decision an earlier phase recorded.
#[test]
fn the_discuss_researchers_read_their_packs_and_search() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    for (agent, subject, carries) in [
        (
            "gsd-assumptions-analyzer",
            "v1.0/02",
            Some("v1.0/02/02-CONTEXT"),
        ),
        (
            "gsd-assumptions-analyzer.compact",
            "v1.0/02",
            Some("v1.0/02/02-CONTEXT"),
        ),
        (
            "gsd-advisor-researcher",
            "v1.0/02",
            Some("v1.0/02/02-CONTEXT"),
        ),
        (
            "gsd-advisor-researcher.compact",
            "v1.0/02",
            Some("v1.0/02/02-CONTEXT"),
        ),
        ("gsd-research-synthesizer", "taskboard", None),
        ("gsd-roadmapper", "taskboard", None),
    ] {
        let role = agent
            .trim_end_matches(".compact")
            .trim_start_matches("gsd-");
        let line = read_of(agent, &format!("specforge context gsd/{role}"), subject);
        let (out, err, code) = sh.run_in(p.root(), &line);
        assert_eq!(code, 0, "{agent}: {line}: {err}");
        assert!(!out.trim().is_empty(), "{agent}: {line} printed no pack");
        if let Some(id) = carries {
            assert!(out.contains(id), "{agent}: the pack lacks {id}:\n{out}");
        }
    }
    for agent in [
        "gsd-assumptions-analyzer",
        "gsd-assumptions-analyzer.compact",
    ] {
        let search = read_of(agent, "specforge search", "renamed");
        let (out, err, code) = sh.run_in(p.root(), &search);
        assert_eq!(code, 0, "{agent}: {search}: {err}");
        assert!(out.contains("v1.0/02/D-02"), "{agent}: {search}:\n{out}");
        let anchors = read_of(agent, "specforge anchors <", "v1.0/02/D-02");
        let (_, err, code) = sh.run_in(p.root(), &anchors);
        assert_eq!(code, 0, "{agent}: {anchors}: {err}");
        let by_file = read_of(agent, "specforge anchors --file", "src");
        let (_, err, code) = sh.run_in(p.root(), &by_file);
        assert_eq!(code, 0, "{agent}: {by_file}: {err}");
    }
}

/// The project researcher's pack line, as new-project's and new-milestone's researcher spawns name it, runs on the
/// taskboard and prints the pack.
#[test]
fn the_project_researcher_reads_its_pack() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    for workflow in [
        "gsd-core/workflows/new-project/detail/elaboration.md",
        "gsd-core/workflows/new-milestone.md",
    ] {
        let text = asset(workflow);
        let line = text
            .lines()
            .map(str::trim_start)
            .find_map(|l| l.strip_prefix("- specforge context gsd/project-researcher "))
            .unwrap_or_else(|| panic!("{workflow}: no project researcher pack line"));
        let subject = line.split_whitespace().next().expect("a subject");
        assert_eq!(subject, "{project}", "{workflow}: {line}");
        let (out, err, code) = sh.run_in(
            p.root(),
            "specforge context gsd/project-researcher taskboard",
        );
        assert_eq!(code, 0, "{workflow}: {err}");
        assert!(out.contains("taskboard"), "{workflow}: {out}");
    }
}
