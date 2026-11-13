//! The write ledger's test: the lines that tell an agent to write a GSD file, counted over every asset.

/// The write ledger (ADR 0332 D9, ADR 0343 D10): every line of every asset, kept or own, that tells an agent to write a
/// GSD file is a row of `markdown_writes.txt`; a row with no hit fails, so the ledger only shrinks.
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
    let hits: Vec<Vec<String>> = super::specforge_only::shell_assets()
        .iter()
        .flat_map(|(path, lines)| super::port_map::markdown_writes::rows_of(path, lines))
        .collect();
    super::ledger::assert_rows(
        "markdown_writes.txt",
        "record the value as a typed edit",
        &hits,
        super::ledger::rows(super::port_map::markdown_writes::MARKDOWN_WRITES, 2),
    );
}

/// The discuss, spec and project family: the assets whose write rows are all gone.
const FAMILY: &str = r"^(commands/gsd/(discuss-phase|spec-phase|new-project|new-milestone|ingest-docs)\.md|gsd-core/workflows/(discuss-phase|discuss-phase-assumptions|discuss-phase-power|spec-phase|new-project|new-milestone|ingest-docs)(\.md|/)|gsd-core/references/(questioning|autonomous-smart-discuss)\.md|gsd-core/templates/((context|project|requirements|roadmap|state|spec)\.md|research-project/)|agents/gsd-(roadmapper|research-synthesizer|doc-synthesizer|doc-classifier|project-researcher|advisor-researcher|assumptions-analyzer)(\.compact)?\.md)";

/// No line of the family tells an agent to generate a GSD file: the write ledger does not count "generate", so a
/// line like "generate CONTEXT.md" would pass its test, and this one holds the family to it.
#[test]
fn no_family_line_tells_an_agent_to_generate_a_gsd_file() {
    use super::port_map::markdown_writes::generates;
    assert_eq!(generates("Generate CONTEXT.md as normal."), [0].into());
    assert!(generates("Record the phase context.").is_empty());
    assert!(generates("```bash\necho generate > STATE.md\n```").is_empty());
    let family = regex::Regex::new(FAMILY).expect("a regex");
    let mut seen = 0;
    let mut hits = Vec::new();
    for (path, lines) in super::specforge_only::shell_assets() {
        if !family.is_match(&path) {
            continue;
        }
        seen += 1;
        for n in generates(&lines.join("\n")) {
            hits.push(format!("{path}: {}", lines[n].trim()));
        }
    }
    assert!(seen > 50, "the family's assets: {seen}");
    assert!(hits.is_empty(), "{hits:#?}");
}
