//! `specforge hook <short>.<id> --host <host> --event <event> [--tools …] [--paths …]` (ADR 0292): the command a
//! host's hook runs. The host's event is read from stdin through the host's adapter, filtered by the flags before
//! anything is loaded, and, if it is one the hook fires on, answered by the hook in the project the event's `cwd`
//! is in; the answer is written in the host's protocol. A hook never fails its host: whatever goes wrong answers by the
//! hook's own policy (`on_failure`; the spec guard denies), and allows only where no hook of the project is known to
//! object: outside a project, or for a route the project does not declare.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use crate::outcome::Exit;
use crate::terminal::{Terminal, err, errln, out};
use specforge_ops::authoring::hook::{self, Filter};
use specforge_ops::host::{Host, HostHookEvent, HostRenderer, HostReply};
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_protocol_types::HookEvent;

/// The most stdin a host may send; more is a failure of the hook, not a truncated event.
pub(crate) const STDIN_LIMIT: u64 = 4 * 1024 * 1024;

/// `specforge hook`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The hook: the extension's short name, a dot and the hook's id (`gsd.write_guard`)
    route: String,

    /// The host that ran it: claude-code, codex or cursor
    #[arg(long)]
    host: String,

    /// The event the hook is wired to: pre_tool, post_tool, pre_commit, user_prompt, session_start, stop,
    /// subagent_start, subagent_stop, pre_compact or statusline
    #[arg(long)]
    event: String,

    /// Tool classes the hook fires on, comma-separated (empty: any tool)
    #[arg(long, value_delimiter = ',')]
    tools: Vec<String>,

    /// A project-relative glob a path of the tool's input must match; repeat it for several (none: any path)
    #[arg(long)]
    paths: Vec<String>,
}

impl Args {
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let mut stdin = Vec::new();
        // One byte past the limit says it was passed; a stdin that cannot be read is what was read of it.
        let _ = std::io::stdin()
            .take(STDIN_LIMIT + 1)
            .read_to_end(&mut stdin);
        let cwd = std::env::current_dir().unwrap_or_default();
        self.run_with(&stdin, &cwd, crate::project::compile_dir, terminal)
    }

    /// Over `stdin`; `compile` loads the project at a root, and is called only for an event the hook fires on.
    pub(crate) fn run_with(
        self,
        stdin: &[u8],
        cwd: &Path,
        compile: impl FnOnce(&Path) -> CompiledProject,
        terminal: &Terminal,
    ) -> Exit {
        let (Some(host), Some(event)) =
            (Host::from_flag(&self.host), HookEvent::parse(&self.event))
        else {
            errln!(
                terminal,
                "error: unknown host '{}' or event '{}'",
                self.host,
                self.event
            );
            return Exit::Unjudged;
        };
        let Some(adapter) = specforge_hosts::renderer(host) else {
            return Exit::Passed;
        };
        let reply = answer(&*adapter, &self, event, stdin, cwd, compile, terminal);
        out!(terminal, "{}", reply.stdout);
        if !reply.stderr.is_empty() {
            err!(terminal, "{}", reply.stderr);
        }
        if reply.exit_code == 0 {
            Exit::Passed
        } else {
            Exit::Unjudged
        }
    }
}

/// The nearest directory at or above `from` that holds `specforge.json`.
fn project_at(from: &Path) -> Option<PathBuf> {
    from.ancestors()
        .find(|dir| dir.join("specforge.json").is_file())
        .map(Path::to_path_buf)
}

fn answer(
    adapter: &dyn HostRenderer,
    args: &Args,
    event: HookEvent,
    stdin: &[u8],
    process_cwd: &Path,
    compile: impl FnOnce(&Path) -> CompiledProject,
    terminal: &Terminal,
) -> HostReply {
    let nothing = HostReply::default();
    let parsed = if stdin.len() as u64 > STDIN_LIMIT {
        Err(format!(
            "the event is over {} MiB",
            STDIN_LIMIT / (1024 * 1024)
        ))
    } else {
        adapter.read_event(stdin).map_err(|why| why.to_string())
    };
    let mut read = match parsed {
        Ok(read) => read,
        Err(why) => {
            errln!(terminal, "hook {}: {why}", args.route);
            // An event that cannot be read cannot be judged: the hook's own policy answers, as for a trap.
            let Some(root) = project_at(process_cwd) else {
                return nothing;
            };
            let project = compile(&root);
            let view = ProjectView::of(&project);
            let event = HostHookEvent {
                event,
                host: args.host.clone(),
                ..HostHookEvent::default()
            };
            let answer = hook::unreadable(&view, &args.route, &why);
            return adapter.write_answer(&event, &answer);
        }
    };
    let cwd = read
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| process_cwd.to_path_buf());
    // The project is found before the filter needs its paths made relative; outside one, the hook is a no-op.
    let Some(root) = project_at(&cwd) else {
        return nothing;
    };
    let filter = Filter {
        event,
        tools: args.tools.clone(),
        paths: args.paths.clone(),
    };
    if !filter.matches(&read, &root, &cwd) {
        return nothing;
    }
    // The event is the hook's: a commit is the host's pre event.
    read.event = event;
    let project = compile(&root);
    let view = ProjectView::of(&project);
    let caller = crate::caller::of(false, crate::outcome::OutputFormat::Human, terminal);
    let outcome = hook::run(&view, &args.route, &read, caller);
    if !outcome.stderr.is_empty() {
        err!(terminal, "{}", outcome.stderr);
    }
    adapter.write_answer(&read, &outcome.answer)
}
