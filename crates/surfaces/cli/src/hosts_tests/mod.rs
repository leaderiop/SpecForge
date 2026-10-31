//! `specforge install`, `uninstall` and `hook` over the GSD-shaped host fixture, served in process (ADR 0292). The
//! helpers [`install`], [`uninstall`] and [`hook`] are the one place a test calls the command line.

mod config;
mod golden;
mod hook;
mod install;
mod safety;

use std::collections::BTreeMap;

use std::sync::Arc;

use clap::Parser;
use serde_json::{Value, json};
use specforge_project::SharedRuntime;
use specforge_project::testing::GsdHosts;

use crate::terminal::{Facts, Terminal};
use specforge_project::testing::{CORE, GSD_HOSTS_EXTENSIONS, TestProject, UI, gsd_hosts_sources};

/// A command's exit code and what it printed.
pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Out {
    /// Stdout as JSON (`--format json`).
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|e| panic!("{e}: {}", self.stdout))
    }

    /// The action install took for `path` on any host: `created`, `updated`, `unchanged` or `removed`.
    pub fn action(&self, path: &str) -> Option<String> {
        self.json()["hosts"]
            .as_array()?
            .iter()
            .flat_map(|h| h["files"].as_array().into_iter().flatten())
            .find(|f| f["path"] == path)
            .and_then(|f| f["action"].as_str().map(str::to_string))
    }
}

/// A project of the fixture's extensions with `core` settings for `@t/core` and `ui` for `@t/ui`.
pub fn project(core: Value, ui: Value) -> TestProject {
    let project = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["settings"] = json!({ CORE: core, UI: ui });
        })
        .enabling(&GSD_HOSTS_EXTENSIONS)
        .installing_enabled();
    for (path, text) in gsd_hosts_sources() {
        project.write(path, text);
    }
    project
}

/// Every file of the project but its sources and the hidden state, by relative path.
pub fn tree(project: &TestProject) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    fn walk(root: &std::path::Path, at: &std::path::Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative.starts_with(".specforge")
                || relative.starts_with(".git")
                || relative == "specforge.json"
                || relative == "specforge.lock"
            {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(relative, std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    walk(project.root(), project.root(), &mut out);
    out
}

#[derive(clap::Parser)]
struct InstallCli {
    #[command(flatten)]
    args: crate::install::InstallArgs,
}

#[derive(clap::Parser)]
struct UninstallCli {
    #[command(flatten)]
    args: crate::install::UninstallArgs,
}

/// The words of a command line: `hosts…`, `flags…`, then the format and the project.
fn words(project: &TestProject, hosts: &[&str], flags: &[&str]) -> Vec<String> {
    let mut argv = vec!["specforge".to_string()];
    argv.extend(hosts.iter().map(ToString::to_string));
    argv.extend(flags.iter().map(ToString::to_string));
    argv.extend([
        "--format".to_string(),
        "json".to_string(),
        "--path".to_string(),
        project.root().display().to_string(),
    ]);
    argv
}

/// Run a command over `project` compiled with the fixture's extensions served in process.
fn run(
    project: &TestProject,
    command: impl FnOnce(&specforge_project::CompiledProject, &Terminal) -> crate::outcome::Exit,
) -> Out {
    let runtime: SharedRuntime = Arc::new(GsdHosts::new().runtime());
    let compiled = project.compile_in(runtime);
    let (terminal, capture) = Terminal::captured("", Facts::PIPED);
    let exit = command(&compiled, &terminal);
    Out {
        code: exit.code(),
        stdout: capture.stdout(),
        stderr: capture.stderr(),
    }
}

/// `specforge install <hosts…> <flags…> --format json` over `project`.
pub fn install(project: &TestProject, hosts: &[&str], flags: &[&str]) -> Out {
    let cli = InstallCli::parse_from(words(project, hosts, flags));
    run(project, |compiled, terminal| {
        cli.args.run_in(compiled, terminal)
    })
}

/// `specforge uninstall <hosts…> <flags…> --format json` over `project`.
pub fn uninstall(project: &TestProject, hosts: &[&str], flags: &[&str]) -> Out {
    let cli = UninstallCli::parse_from(words(project, hosts, flags));
    run(project, |compiled, terminal| {
        cli.args.run_in(compiled, terminal)
    })
}

/// What `specforge hook` did: its exit code and output, and the exports the guest was asked for.
pub struct Hooked {
    pub out: Out,
    pub exports: Vec<String>,
    /// The `__pass_*` calls the run made at all: the compile's checks.
    pub passes: usize,
    /// The input of the last `hook__*` call the guest was sent.
    pub input: Option<Value>,
}

#[derive(clap::Parser)]
struct HookCli {
    #[command(flatten)]
    args: crate::hook::Args,
}

/// `specforge hook <route> --host <host> --event <event> …` over `project` with `stdin`. The project is compiled only
/// if the hook asks for it (ADR 0324); `exports` are the calls after the load, a compile's `__pass_*` included.
pub fn hook(project: &TestProject, id: &str, host: &str, args: &[&str], stdin: &Value) -> Hooked {
    hook_bytes(project, id, host, args, stdin.to_string().as_bytes())
}

/// [`hook`] over raw `stdin` bytes, run in the project's directory.
pub fn hook_bytes(
    project: &TestProject,
    id: &str,
    host: &str,
    args: &[&str],
    stdin: &[u8],
) -> Hooked {
    let mut argv = vec![
        "specforge".to_string(),
        id.to_string(),
        "--host".to_string(),
        host.to_string(),
    ];
    argv.extend(args.iter().map(ToString::to_string));
    let cli = HookCli::parse_from(argv);
    let runtime = Arc::new(GsdHosts::new().runtime());
    let compiled_at = std::cell::Cell::new(None);
    let (terminal, capture) = Terminal::captured("", Facts::PIPED);
    let exit = cli.args.run_with(
        stdin,
        project.root(),
        |_| {
            let env = project.environment_in(Arc::clone(&runtime) as SharedRuntime);
            compiled_at.set(Some(runtime.calls().len()));
            env
        },
        &terminal,
    );
    let exports = compiled_at
        .get()
        .map(|n| {
            runtime
                .calls()
                .into_iter()
                .skip(n)
                .map(|c| c.export)
                .collect()
        })
        .unwrap_or_default();
    Hooked {
        out: Out {
            code: exit.code(),
            stdout: capture.stdout(),
            stderr: capture.stderr(),
        },
        exports,
        passes: runtime
            .calls()
            .iter()
            .filter(|c| c.export.starts_with("__pass_"))
            .count(),
        input: runtime
            .calls()
            .into_iter()
            .rev()
            .find(|c| c.export.starts_with("hook__"))
            .map(|c| c.input),
    }
}

/// `@t/ui`'s name, as a setting block key.
pub const UI_EXTENSION: &str = UI;
