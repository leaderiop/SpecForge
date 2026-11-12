//! The SpecForge tools a contribution is granted (ADR 0334): its `specforge:` classes resolved against the engine-tool
//! catalogue and the commands the project serves, or the default when it names none. The adapters name what this
//! resolves, and the generated section ([`super::sections`]) lists it.

use specforge_protocol_types::ToolClass;
use specforge_protocol_types::engine_tools::{
    CLASS_PREFIX, ENGINE_TOOLS, EngineClass, EngineGroup, EngineTool, engine_tool, parse_class,
};
use specforge_registry::RegistryBuild;

/// One resolved tool, as the generated section writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineLine {
    /// Its MCP tool's name (`specforge.trace`, `specforge.gsd.record_verdict`).
    pub mcp: String,
    /// Its CLI lines, each starting `specforge `; empty when only MCP serves it.
    pub cli: Vec<String>,
    /// When to use it, in one sentence.
    pub use_when: String,
}

impl EngineLine {
    /// The line of an engine tool.
    pub fn of(tool: &EngineTool) -> Self {
        EngineLine {
            mcp: tool.mcp_name(),
            cli: tool.cli.iter().map(|l| format!("specforge {l}")).collect(),
            use_when: tool.use_when.to_string(),
        }
    }
}

/// A command the project serves an agent: neither human-attested nor refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedCommand {
    /// The declaring extension's name.
    pub extension: String,
    pub line: EngineLine,
}

impl ServedCommand {
    /// Its `specforge:<short>.<id>` class.
    fn class(&self) -> String {
        let named = self
            .line
            .mcp
            .strip_prefix("specforge.")
            .unwrap_or(&self.line.mcp);
        format!("{CLASS_PREFIX}{named}")
    }
}

/// The first sentence of `text`, ending with a full stop.
fn first_sentence(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default();
    let sentence = line.split_once(". ").map_or(line, |(first, _)| first);
    format!("{}.", sentence.trim().trim_end_matches('.'))
}

/// The commands `build` routes that an agent may run, in load order: each by its MCP tool, its CLI line and the
/// first sentence of its description.
pub fn served_commands(build: &RegistryBuild) -> Vec<ServedCommand> {
    build
        .commands()
        .routed()
        .iter()
        .filter(|r| r.declaration().unrunnable().is_none())
        .map(|r| {
            let c = r.declaration();
            let text = if c.description.trim().is_empty() {
                &c.title
            } else {
                &c.description
            };
            ServedCommand {
                extension: r.extension().to_string(),
                line: EngineLine {
                    mcp: format!("specforge.{}.{}", r.short(), c.id),
                    cli: vec![format!("specforge {} {}", r.short(), r.cli_name())],
                    use_when: first_sentence(text),
                },
            }
        })
        .collect()
}

/// What a contribution is granted: the classes an adapter names, and the lines its section lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// Its host classes, then `specforge:<tool>` per engine tool in catalogue order, then `specforge:<short>.<id>`
    /// per command by MCP name; empty only for an empty declaration with no target (the host's whole set).
    pub classes: Vec<ToolClass>,
    /// Every resolved tool's line, in the order of `classes`.
    pub lines: Vec<EngineLine>,
}

/// The SpecForge tools of a contribution of `extension` that declares `declared`: the engine tools and served commands
/// its `specforge:` classes name, or, when it names none, the reads and navigation tools, the edits and output tools
/// when a host class writes, and its extension's own commands; and the class `target` (a slash command's trampoline)
/// besides. A class the registry kept that names no served command is skipped.
pub fn resolve(
    declared: &[ToolClass],
    extension: &str,
    served: &[ServedCommand],
    target: Option<&str>,
) -> Resolved {
    let host: Vec<ToolClass> = declared
        .iter()
        .filter(|c| !c.starts_with(CLASS_PREFIX))
        .cloned()
        .collect();
    let mut rows = [false; ENGINE_TOOLS.len()];
    let mut commands: Vec<&ServedCommand> = Vec::new();
    let mut take = |class: &str| match parse_class(class) {
        Some(EngineClass::Tool(tool)) => {
            if let Some(i) = ENGINE_TOOLS.iter().position(|t| t.name == tool.name) {
                rows[i] = true;
            }
        }
        Some(EngineClass::Group(group)) => {
            for (i, tool) in ENGINE_TOOLS.iter().enumerate() {
                rows[i] |= tool.group == group;
            }
        }
        Some(EngineClass::Command { short, id }) => {
            let mcp = format!("specforge.{short}.{id}");
            commands.extend(served.iter().find(|c| c.line.mcp == mcp));
        }
        None => {}
    };
    let named: Vec<&ToolClass> = declared
        .iter()
        .filter(|c| c.starts_with(CLASS_PREFIX))
        .collect();
    if named.is_empty() {
        let writes = host.iter().any(|c| c == "write" || c == "edit");
        let mut groups = vec![EngineGroup::Reads, EngineGroup::Navigation];
        if writes {
            groups.extend([EngineGroup::Edits, EngineGroup::Output]);
        }
        for group in groups {
            take(&format!("{CLASS_PREFIX}{}", group.as_str()));
        }
        for own in served.iter().filter(|c| c.extension == extension) {
            take(&own.class());
        }
    } else {
        for class in named {
            take(class);
        }
    }
    if let Some(class) = target {
        take(class);
    }
    commands.sort_by(|a, b| a.line.mcp.cmp(&b.line.mcp));
    commands.dedup_by(|a, b| a.line.mcp == b.line.mcp);

    let tools: Vec<&EngineTool> = ENGINE_TOOLS
        .iter()
        .zip(rows)
        .filter_map(|(tool, on)| on.then_some(tool))
        .collect();
    let mut lines: Vec<EngineLine> = tools.iter().map(|t| EngineLine::of(t)).collect();
    lines.extend(commands.iter().map(|c| c.line.clone()));
    if declared.is_empty() && target.is_none() {
        return Resolved {
            classes: Vec::new(),
            lines,
        };
    }
    let mut classes = host;
    if lines.is_empty() {
        // A host that is given no list grants every tool: a declaration never resolves to none.
        let query = engine_tool("query").expect("the catalogue has query");
        classes.push(format!("{CLASS_PREFIX}{}", query.name));
        lines.push(EngineLine::of(query));
        return Resolved { classes, lines };
    }
    classes.extend(tools.iter().map(|t| format!("{CLASS_PREFIX}{}", t.name)));
    classes.extend(commands.iter().map(|c| c.class()));
    Resolved { classes, lines }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(c: &[&str]) -> Vec<ToolClass> {
        c.iter().map(|t| (*t).to_string()).collect()
    }

    fn command(extension: &str, short: &str, id: &str) -> ServedCommand {
        ServedCommand {
            extension: extension.into(),
            line: EngineLine {
                mcp: format!("specforge.{short}.{id}"),
                cli: vec![format!("specforge {short} {}", id.replace('_', "-"))],
                use_when: "Do it.".into(),
            },
        }
    }

    fn served() -> Vec<ServedCommand> {
        vec![
            command("@t/core", "t", "stats"),
            command("@t/core", "t", "approve_all"),
            command("@t/ui", "t", "paint"),
        ]
    }

    fn of_group(group: EngineGroup) -> Vec<String> {
        ENGINE_TOOLS
            .iter()
            .filter(|t| t.group == group)
            .map(|t| format!("specforge:{}", t.name))
            .collect()
    }

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "a contribution's engine tool classes resolve in catalogue order, a group to each of its tools"
    )]
    fn named_classes_resolve_in_catalogue_order_and_a_group_to_its_rows() {
        let declared = classes(&[
            "read",
            "specforge:t.stats",
            "specforge:navigation",
            "specforge:trace",
            "search",
        ]);
        let r = resolve(&declared, "@t/core", &served(), None);
        let mut want = classes(&["read", "search", "specforge:trace"]);
        want.extend(of_group(EngineGroup::Navigation));
        want.push("specforge:t.stats".into());
        assert_eq!(r.classes, want);
        assert_eq!(r.lines.len(), want.len() - 2);
        assert_eq!(r.lines[0].mcp, "specforge.trace");
        assert_eq!(
            r.lines.last().map(|l| l.mcp.as_str()),
            Some("specforge.t.stats")
        );
    }

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "a contribution that names no engine tool class gets the read and navigation tools, the edit and output tools when it writes, and its extension's own commands"
    )]
    fn a_contribution_naming_no_engine_class_gets_the_default_tools() {
        let reader = resolve(&classes(&["read", "search"]), "@t/core", &served(), None);
        let mut want = classes(&["read", "search"]);
        want.extend(of_group(EngineGroup::Reads));
        want.extend(of_group(EngineGroup::Navigation));
        // Catalogue order interleaves the groups (verdict_inputs and infer_* sit after the setup rows).
        let order = |c: &String| {
            ENGINE_TOOLS
                .iter()
                .position(|t| c.strip_prefix("specforge:") == Some(t.name))
        };
        want[2..].sort_by_key(order);
        want.extend(classes(&["specforge:t.approve_all", "specforge:t.stats"]));
        assert_eq!(reader.classes, want);
        assert!(!reader.classes.contains(&"specforge:edit".to_string()));
        assert!(!reader.classes.contains(&"specforge:t.paint".to_string()));

        let writer = resolve(&classes(&["read", "edit"]), "@t/ui", &served(), None);
        assert!(writer.classes.contains(&"specforge:edit".to_string()));
        assert!(writer.classes.contains(&"specforge:collect".to_string()));
        assert!(writer.classes.contains(&"specforge:t.paint".to_string()));
        assert!(!writer.classes.contains(&"specforge:t.stats".to_string()));
        for setup in of_group(EngineGroup::Setup) {
            assert!(!writer.classes.contains(&setup), "{setup}");
        }
    }

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "an agent or skill that declares no tools keeps the host's whole tool set, and its section lists the default SpecForge tools"
    )]
    fn an_agent_with_no_tools_keeps_the_hosts_set_and_lists_the_default() {
        let r = resolve(&[], "@t/core", &served(), None);
        assert!(r.classes.is_empty());
        let default = resolve(&classes(&["read"]), "@t/core", &served(), None);
        assert_eq!(r.lines, default.lines);
    }

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "a slash command also gets the SpecForge tool its trampoline calls"
    )]
    fn a_slash_command_also_gets_the_tool_it_calls() {
        let prompt = resolve(
            &classes(&["read", "specforge:trace"]),
            "@t/core",
            &served(),
            Some("specforge:prompt"),
        );
        assert_eq!(
            prompt.classes,
            classes(&["read", "specforge:trace", "specforge:prompt"])
        );
        let command = resolve(&[], "@t/ui", &served(), Some("specforge:t.stats"));
        assert!(command.classes.contains(&"specforge:t.stats".to_string()));
        assert!(command.classes.contains(&"specforge:query".to_string()));
    }

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "a human-attested or refused extension command is never granted"
    )]
    fn a_human_or_refused_command_is_never_granted() {
        // `served_commands` leaves them out; a class naming one resolves to nothing for it.
        let r = resolve(
            &classes(&["read", "specforge:t.approve", "specforge:trace"]),
            "@t/core",
            &served(),
            None,
        );
        assert_eq!(r.classes, classes(&["read", "specforge:trace"]));
    }

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "a non-empty tool declaration that resolves to no SpecForge tool keeps its host classes and specforge:query, never an empty list"
    )]
    fn a_declaration_that_resolves_to_nothing_never_writes_an_empty_list() {
        let r = resolve(
            &classes(&["read", "specforge:t.gone"]),
            "@t/core",
            &served(),
            None,
        );
        assert_eq!(r.classes, classes(&["read", "specforge:query"]));
        assert_eq!(r.lines.len(), 1);
        let only = resolve(&classes(&["specforge:t.gone"]), "@t/core", &served(), None);
        assert_eq!(only.classes, classes(&["specforge:query"]));
    }

    #[test]
    fn a_commands_line_is_the_first_sentence_of_its_description() {
        assert_eq!(
            first_sentence("Record a verdict. Then more."),
            "Record a verdict."
        );
        assert_eq!(first_sentence("No stop\nsecond line"), "No stop.");
        assert_eq!(first_sentence("Done."), "Done.");
    }
}
