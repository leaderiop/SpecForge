//! `specforge hook <short>.<id> --host <host> --event <event> [--tools …] [--paths …]` (ADR 0292): the command a
//! host's hook runs. The host's event is read from stdin through the host's adapter, filtered by the flags before
//! anything is loaded, and, if it is one the hook fires on, answered by the hook in the project the event's `cwd`
//! is in; the answer is written in the host's protocol. A hook never fails its host: whatever goes wrong answers by the
//! hook's own policy (`on_failure`; the spec guard denies), and allows only where no hook of the project is known to
//! object: outside a project, or for a route the project does not declare. A hook that reads no graph and writes
//! nothing is answered over the loaded environment, without reading the project's sources (ADR 0324).

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::outcome::Exit;
use crate::terminal::{Terminal, err, errln, out};
use specforge_ops::authoring::hook::{self, Filter};
use specforge_ops::host::render::SPEC_GUARD;
use specforge_ops::host::{Host, HostHookEvent, HostRenderer, HostReply};
use specforge_ops::view::ProjectView;
use specforge_project::{CompiledProject, Environment};
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
    /// subagent_start, subagent_stop, pre_compact, statusline or file_changed
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
        // A hook's git facts degrade quietly after 1.5 s, as upstream's probes do (ADR 0320).
        specforge_project::history::set_default(Some(std::sync::Arc::new(
            specforge_git::GitCli::with_timeout(std::time::Duration::from_millis(1_500)),
        )));
        self.run_with(&stdin, &cwd, crate::project::load_dir, terminal)
    }

    /// Over `stdin`; `load` loads the environment at a root, and is called only for an event the hook fires on.
    pub(crate) fn run_with(
        self,
        stdin: &[u8],
        cwd: &Path,
        load: impl FnOnce(&Path) -> Environment,
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
            // Exit 2 is the hook's own deny; a refusal of the binary must not read as one (ADR 0324).
            return Exit::Failed;
        };
        let Some(adapter) = specforge_hosts::renderer(host) else {
            return Exit::Passed;
        };
        let reply = answer(&*adapter, &self, event, stdin, cwd, load, terminal);
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
    load: impl FnOnce(&Path) -> Environment,
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
            let project = CompiledProject::unread(Arc::new(load(&root)));
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
    let env = load(&root);
    // The spec guard reads the graph for the spec root; an unknown route is answered from the registries alone.
    let compile = args.route == SPEC_GUARD
        || env
            .registries()
            .hook(&args.route)
            .is_some_and(|h| h.item.reads_graph || h.item.writes);
    let project = if compile {
        CompiledProject::of(env)
    } else {
        CompiledProject::unread(Arc::new(env))
    };
    let view = ProjectView::of(&project);
    let caller = crate::caller::of(false, crate::outcome::OutputFormat::Human, terminal);
    let outcome = hook::run(&view, &args.route, &read, caller);
    if !outcome.stderr.is_empty() {
        err!(terminal, "{}", outcome.stderr);
    }
    adapter.write_answer(&read, &outcome.answer)
}
