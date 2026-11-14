//! No asset tells an agent to write a GSD file: a write is a typed edit (ADR 0343 D10).

/// No line of any asset of either tree, kept or own, tells an agent to write a GSD file, to edit a state file or a
/// part of one ("Update Tests section", "record overrides in MILESTONES.md"), or to generate one ("generate
/// CONTEXT.md"): the write, section and generate detectors all find nothing.
#[test]
fn no_line_tells_an_agent_to_write_a_gsd_file() {
    let quoted = format!(
        "[CONTEXT.md]({}CONTEXT.md) is updated upstream",
        gsd_kit::files::UPSTREAM_BLOB
    );
    for (text, hit) in [
        ("Write `STATE.md` with the new position.", true),
        ("Do not create SUMMARY.md.", true),
        ("Read `ROADMAP.md`.", false),
        (
            "```bash\nprintf '%s' \"$X\" > \"$DIR/03-CONTEXT.md\"\n```",
            true,
        ),
        (
            "```bash\nspecforge gsd x | tee \"${PHASE}-UAT.md\"\n```",
            true,
        ),
        (
            "```bash\nprintf '%s' \"$X\" >> \"$DIR/STATE.md\"\n```",
            true,
        ),
        (
            "```bash\nprintf '%s' \"$X\" > \"$TMP/reqs.json\"\n```",
            false,
        ),
        (
            "```bash\nwhile read l; do :; done < \"$P/03-CONTEXT.md\"\n```",
            false,
        ),
        ("```bash\n# write STATE.md here\n```", false),
        (
            "If \"View\": Display CONTEXT.md, then offer update/skip",
            false,
        ),
        ("- Review/edit CONTEXT.md before continuing", false),
        ("Ask the user to edit ROADMAP.md.", false),
        (
            "Spawned by /gsd:plan-phase orchestrator (after planner creates PLAN.md)",
            true,
        ),
        (
            "complete item, updates STATE.md, and prints the final batch report.",
            true,
        ),
        (quoted.as_str(), false),
        ("```markdown\nUpdate STATE.md with the plan.\n```", true),
    ] {
        assert_eq!(
            super::port_map::markdown_writes::writes(text),
            hit,
            "{text}"
        );
    }
    use super::port_map::markdown_writes::{generates, section_edits};
    assert_eq!(generates("Generate CONTEXT.md as normal."), [0].into());
    assert!(generates("Record the phase context.").is_empty());
    assert!(generates("```bash\necho generate > STATE.md\n```").is_empty());
    for (text, hit) in [
        ("Update Tests section:", true),
        (
            "Append to Gaps section (structured YAML for plan-phase --gaps):",
            true,
        ),
        ("Update frontmatter.updated timestamp.", true),
        (
            "Record acknowledged gaps in VERIFICATION.md `## Acknowledged Gaps` section.",
            true,
        ),
        (
            "note incomplete requirements in MILESTONES.md under `### Known Gaps`",
            false,
        ),
        ("- [ ] Known gaps recorded in MILESTONES.md", true),
        ("Record each root cause on its `uat_gap`.", false),
        ("This section adds only the surrounding scaffolding.", false),
        ("append the following audit section.", false),
        ("Ask the user to record it in STATE.md.", false),
        ("To accept it, add an override to VERIFICATION.md.", true),
        ("Produce PLAN.md files that executors can implement.", true),
        (
            "The plan must produce `SKELETON.md` in the phase directory.",
            true,
        ),
        ("In SUMMARY.md, document auth gates as normal flow.", false),
        (
            "REVIEWS.md (produced by /gsd:review) holds the feedback.",
            false,
        ),
        (
            "PLAN.md IS the prompt (not a document that becomes one).",
            false,
        ),
        ("produces: PLAN.md", false),
        ("If CONTEXT.md exists, add verification dimension.", false),
    ] {
        assert_eq!(!section_edits(text).is_empty(), hit, "{text}");
    }
    assert_eq!(
        generates(
            "When the user signals readiness, processes all answers and generates CONTEXT.md."
        ),
        [0].into()
    );
    assert!(generates("If the user agrees, the user generates CONTEXT.md.").is_empty());
    let mut seen = 0;
    let mut hits = Vec::new();
    for (path, lines) in super::specforge_only::shell_assets() {
        seen += 1;
        let text = lines.join("\n");
        for n in section_edits(&text).union(&generates(&text)) {
            hits.push(format!("{path}:{} {}", n + 1, lines[*n].trim()));
        }
    }
    assert!(seen > 300, "the assets: {seen}");
    assert!(
        hits.is_empty(),
        "a write is a typed edit (ADR 0343 D10):\n{}",
        hits.join("\n")
    );
}
