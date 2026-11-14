//! The state-file guard: no `specforge files` run names a GSD file, which is an entity.

/// The words of the shell `line`'s `specforge files <verb>` commands that name a GSD state file
/// (`gsd_kit::files::planning_names`), read before the line's comment by the SDK shell reader.
fn files_reads_of_gsd_files(line: &str) -> Result<Vec<String>, String> {
    Ok(super::specforge_only::commands_before_comment(line)?
        .into_iter()
        .filter(|words| words.len() > 2 && words[0] == "specforge" && words[1] == "files")
        .flat_map(|words| words.into_iter().skip(3))
        .filter(|word| !gsd_kit::files::planning_names(word).is_empty())
        .collect())
}

/// No `specforge files` run, in a shell fence (a `\`-continued line joined to the next) or an inline run, names a GSD
/// state file (ADR 0343 D3): the file is an entity, so a `files` verb that looks for it finds nothing and the step
/// takes the "absent" route.
#[test]
fn no_files_verb_reads_a_gsd_file() {
    let fence_line = "specforge files list --kind file --no-ignore --glob '*-CONTEXT.md' 2>/dev/null || true   # the next phase's";
    assert_eq!(
        files_reads_of_gsd_files(fence_line),
        Ok(vec!["*-CONTEXT.md".to_string()]),
        "a fence line, its comment cut"
    );
    let inline: Vec<String> =
        gsd_kit::markdown::inline_code("Resolve `specforge files stat \"x/01-UAT.md\"` first.")
            .into_iter()
            .map(|(_, run)| run)
            .collect();
    assert_eq!(inline.len(), 1, "{inline:?}");
    assert_eq!(
        files_reads_of_gsd_files(&inline[0]),
        Ok(vec!["x/01-UAT.md".to_string()]),
        "an inline run"
    );
    let mut found = Vec::new();
    let mut runs = 0;
    for (path, lines) in super::specforge_only::shell_assets() {
        let fenced = super::specforge_only::shell_blocks(&lines)
            .into_iter()
            .flat_map(|block| super::port_map::tool_names::shell_lines(&block.join("\n")));
        let inline = super::specforge_only::inline_runs(&lines)
            .into_iter()
            .map(|(_, run)| run);
        for line in fenced
            .chain(inline)
            .filter(|l| l.contains("specforge files"))
        {
            runs += 1;
            let words = files_reads_of_gsd_files(&line).unwrap_or_else(|e| panic!("{path}: {e}"));
            if !words.is_empty() {
                found.push(format!("{path}: {} names {words:?}", line.trim()));
            }
        }
    }
    assert!(runs > 20, "only {runs} files runs read");
    assert!(
        found.is_empty(),
        "{} files runs read a GSD state file; read its entity:\n{}",
        found.len(),
        found.join("\n")
    );
}
