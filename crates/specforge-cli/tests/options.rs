//! Each enumerated argument on both surfaces (architecture plan 2026-10-06
//! 12): the names the CLI and MCP accept for it, the name an absent
//! argument selects, and what each surface advertises (the CLI's `-h`
//! line, the MCP input schema's `enum` and `default`).
//!
//! `enumerated_options_today` pins what the surfaces answer today, drifts
//! included, as one insta snapshot over a scratch copy of
//! `fixtures/read_views/rv1`. It proves no spec obligation (it pins current
//! behaviour, bugs included), so it carries no `specforge_test` link; a
//! change that alters what it pins re-blesses the snapshot in the same
//! commit, where the diff shows it.

use std::fmt::Write as _;
use std::path::Path;

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::coverage_corpus::{copy_tree, mcp_calls};

/// A scratch copy of `fixtures/read_views/rv1`.
fn rv1() -> TempDir {
    let tmp = TempDir::new().unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read_views/rv1"),
        tmp.path(),
    );
    tmp
}

/// The placeholders of a probe's CLI command line.
const ROOT: &str = "{root}";
const NAME: &str = "{name}";

/// One enumerated argument, probed on each surface it has.
struct Probe {
    label: &'static str,
    /// The CLI command line, `{root}` and `{name}` filled in; an absent
    /// argument drops `{name}` and the flag before it.
    cli: Option<&'static [&'static str]>,
    mcp: Option<McpProbe>,
    names: &'static [&'static str],
}

/// The MCP tool and argument a probe calls, with the other arguments every
/// call passes.
struct McpProbe(&'static str, &'static str, fn() -> Value);

fn no_arguments() -> Value {
    json!({})
}

fn entity_alpha() -> Value {
    json!({"entity_id": "alpha"})
}

const PROBES: &[Probe] = &[
    Probe {
        label: "export format",
        cli: Some(&["export", ROOT, "--format", NAME]),
        mcp: Some(McpProbe("specforge.export", "format", no_arguments)),
        names: &["graph", "context", "brief", "dot", "json", "yaml"],
    },
    Probe {
        label: "schema publish format",
        cli: Some(&["schema", ROOT, "--publish", "--format", NAME]),
        mcp: None,
        names: &["graph", "context", "brief", "dot", "json"],
    },
    Probe {
        label: "model format",
        cli: Some(&["model", ROOT, "--format", NAME]),
        mcp: Some(McpProbe("specforge.model", "format", no_arguments)),
        names: &["markdown", "mermaid", "dot", "json", "dbml", "svg"],
    },
    Probe {
        label: "model group_by",
        cli: Some(&["model", ROOT, "--group-by", NAME]),
        mcp: Some(McpProbe("specforge.model", "group_by", no_arguments)),
        names: &["extension", "none", "flat"],
    },
    Probe {
        label: "model fields",
        cli: Some(&["model", ROOT, "--fields", NAME]),
        mcp: Some(McpProbe("specforge.model", "fields", no_arguments)),
        names: &["none", "keys", "all", "some"],
    },
    Probe {
        label: "outline format",
        cli: Some(&["outline", ROOT, "--format", NAME]),
        mcp: Some(McpProbe(
            "specforge.outline_extensions",
            "format",
            no_arguments,
        )),
        names: &["markdown", "mermaid", "dot", "json", "svg"],
    },
    Probe {
        label: "outline fields",
        cli: Some(&["outline", ROOT, "--fields", NAME]),
        mcp: Some(McpProbe(
            "specforge.outline_extensions",
            "fields",
            no_arguments,
        )),
        names: &["none", "keys", "all"],
    },
    Probe {
        label: "outline deps",
        cli: Some(&["outline", ROOT, "--deps", NAME]),
        mcp: Some(McpProbe(
            "specforge.outline_extensions",
            "deps",
            no_arguments,
        )),
        names: &["direct", "effective", "full", "transitive"],
    },
    Probe {
        label: "render format",
        cli: None,
        mcp: Some(McpProbe("specforge.render", "format", no_arguments)),
        names: &["json", "graph", "dot", "context", "brief", "yaml"],
    },
    Probe {
        label: "query format",
        cli: None,
        mcp: Some(McpProbe("specforge.query", "format", entity_alpha)),
        names: &["graph", "context", "brief", "json", "dot", "yaml"],
    },
    Probe {
        label: "coverage status_filter",
        cli: None,
        mcp: Some(McpProbe(
            "specforge.coverage",
            "status_filter",
            no_arguments,
        )),
        names: &["covered", "partial", "uncovered", "coverd"],
    },
    Probe {
        label: "find_references direction",
        cli: None,
        mcp: Some(McpProbe(
            "specforge.find_references",
            "direction",
            entity_alpha,
        )),
        names: &["incoming", "outgoing", "both", "sideways"],
    },
    Probe {
        label: "analyze pass",
        cli: Some(&["analyze", NAME, "--path", ROOT]),
        mcp: Some(McpProbe("specforge.analyze", "pass", no_arguments)),
        names: &[
            "all",
            "coverage",
            "contracts",
            "@specforge/testing:coverage",
            "nosuch",
        ],
    },
];

/// What a CLI command printed, and its exit code.
struct Run {
    code: Option<i32>,
    stdout: String,
}

fn cli(args: &[String]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_specforge"))
        .args(args)
        .output()
        .unwrap();
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
    }
}

/// `line` with `{root}` and `{name}` filled in; with no name, `{name}` and
/// the flag before it are dropped.
fn command_line(line: &[&str], root: &Path, name: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    for token in line {
        match *token {
            ROOT => args.push(root.to_str().unwrap().to_string()),
            NAME => match name {
                Some(name) => args.push(name.to_string()),
                None => {
                    if args.last().is_some_and(|flag| flag.starts_with("--")) {
                        args.pop();
                    }
                }
            },
            other => args.push(other.to_string()),
        }
    }
    args
}

/// The `-h` lines of the flag (or positional) `line` passes the name with,
/// joined into one.
fn help_line(line: &[&str]) -> String {
    let at = line.iter().position(|token| *token == NAME).unwrap();
    let previous = line[at - 1];
    let (command, wanted) = if previous.starts_with("--") {
        (line[0], format!("{previous} <"))
    } else {
        (line[0], "[PASS]".to_string())
    };
    let help = cli(&[command.to_string(), "-h".to_string()]).stdout;
    let mut lines = help
        .lines()
        .skip_while(|l| !l.trim_start().starts_with(&wanted));
    let mut joined = lines.next().unwrap_or("").trim().to_string();
    // Help text clap wraps or moves under the flag is indented deeper than
    // any flag (2 or 6 spaces).
    for next in lines {
        let indent = next.len() - next.trim_start().len();
        if next.trim().is_empty() || indent <= 6 {
            break;
        }
        joined.push(' ');
        joined.push_str(next.trim());
    }
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The MCP input schema's `enum` and `default` of `tool`'s `argument`.
fn advertised(tool: &str, argument: &str) -> String {
    let spec = specforge_mcp::tools::CORE_TOOLS
        .iter()
        .find(|spec| spec.name == tool)
        .unwrap_or_else(|| panic!("no tool {tool}"));
    let property = (spec.schema)()["properties"][argument].clone();
    let required = (spec.schema)()["required"]
        .as_array()
        .is_some_and(|names| names.iter().any(|name| name == argument));
    format!(
        "enum {} default {}{}",
        property.get("enum").unwrap_or(&Value::Null),
        property.get("default").unwrap_or(&Value::Null),
        if required { " (required)" } else { "" }
    )
}

/// An MCP result as the snapshot shows it: `ok`, or the refusal's code,
/// message and suggestion.
fn mcp_outcome(result: &Value, root: &Path) -> String {
    let Some(error) = result.get("isError").or_else(|| result.get("error")) else {
        return "ok".to_string();
    };
    let mut text = format!(
        "refused: {}: {}",
        error["code"].as_str().unwrap_or("?"),
        error["message"].as_str().unwrap_or("?")
    );
    let suggestion = error["data"]["suggestion"]
        .as_str()
        .or_else(|| error["diagnostic"]["suggestion"].as_str());
    if let Some(suggestion) = suggestion {
        write!(text, " (suggestion: {suggestion})").unwrap();
    }
    if let Some(data) = error["data"].as_object()
        && let Some(renderers) = data.get("available_renderers")
    {
        write!(text, " (available_renderers: {renderers})").unwrap();
    }
    text.replace(root.to_str().unwrap(), "[ROOT]")
}

/// Which of `names` gave what the absent argument gave.
fn defaults<T: PartialEq>(absent: &T, outputs: &[(&str, T)]) -> String {
    let matching: Vec<&str> = outputs
        .iter()
        .filter(|(_, output)| output == absent)
        .map(|(name, _)| *name)
        .collect();
    if matching.is_empty() {
        "(none)".to_string()
    } else {
        matching.join(" = ")
    }
}

/// One probe's section of the snapshot.
fn probe(probe: &Probe) -> String {
    let tmp = rv1();
    let root = tmp.path();
    let mut section = format!("## {}\n", probe.label);

    let cli_runs = probe.cli.map(|line| {
        writeln!(section, "cli -h: {}", help_line(line)).unwrap();
        let absent = cli(&command_line(line, root, None));
        let runs: Vec<(&str, Run)> = probe
            .names
            .iter()
            .map(|name| (*name, cli(&command_line(line, root, Some(name)))))
            .collect();
        (absent, runs)
    });

    let mcp_results = probe.mcp.as_ref().map(|McpProbe(tool, argument, base)| {
        writeln!(section, "mcp schema: {}", advertised(tool, argument)).unwrap();
        let mut calls = vec![json!({"name": tool, "arguments": base()})];
        for name in probe.names {
            let mut arguments = base();
            arguments[argument] = json!(name);
            calls.push(json!({"name": tool, "arguments": arguments}));
        }
        let mut results = mcp_calls(root, &calls).into_iter();
        let absent = results.next().unwrap();
        let named: Vec<(&str, Value)> = probe.names.iter().copied().zip(results).collect();
        (absent, named)
    });

    let mut default_line = String::from("default:");
    if let Some((absent, runs)) = &cli_runs {
        let stdouts: Vec<(&str, (Option<i32>, &str))> = runs
            .iter()
            .map(|(name, run)| (*name, (run.code, run.stdout.as_str())))
            .collect();
        let absent = (absent.code, absent.stdout.as_str());
        write!(default_line, " cli={}", defaults(&absent, &stdouts)).unwrap();
    }
    if let Some((absent, named)) = &mcp_results {
        write!(default_line, " mcp={}", defaults(absent, named)).unwrap();
    }
    section.push_str(&default_line);
    section.push('\n');

    for (i, name) in probe.names.iter().enumerate() {
        let cli = cli_runs
            .as_ref()
            .map_or("-".to_string(), |(_, runs)| match runs[i].1.code {
                Some(code) => code.to_string(),
                None => "signal".to_string(),
            });
        let mcp = mcp_results
            .as_ref()
            .map_or("-".to_string(), |(_, named)| mcp_outcome(&named[i].1, root));
        writeln!(section, "{name:<28} cli={cli:<3} mcp={mcp}").unwrap();
    }
    section
}

#[test]
fn enumerated_options_today() {
    let sections: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = PROBES
            .iter()
            .map(|p| scope.spawn(move || probe(p)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    insta::assert_snapshot!("enumerated_options_today", sections.join("\n"));
}
