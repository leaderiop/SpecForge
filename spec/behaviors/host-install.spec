// Host install behaviors — an extension contributes agents, skills, slash commands and hooks (ADR 0292)
//
// P7 Justification: a host (Claude Code, Codex, Cursor) is a file format, never a domain. The renderers know
// each host's layout and hook protocol and no word of any extension's domain; every name, body and filter
// they write comes from an extension's declaration.

use "invariants/wasm"

defaults behavior {
  features [contribution_based_extensions]
}

behavior install_into_a_host "Install Into a Host" {
  invariants [extension_isolation]
  category   command
  contract   """
    specforge install <host> renders every agent, skill and slash command the project's extensions contribute, their
    hooks, SpecForge's MCP server entry and its spec guard into the host's project-local files (ADR 0292). It is
    idempotent, all or nothing under the write lock, and owns exactly what it marked: a file it writes starts with a
    generated-by marker holding its source and digest, a config entry it owns is the one marked
    `# specforge-install` (a hook) or named specforge (the MCP server). It never follows a link.
    It refuses to overwrite an unmarked file (foreign_file) or a marked file edited since (edited_file, unless
    forced), removes what no extension contributes any more and reports what a host cannot take. A project whose
    extensions contribute nothing to a host declares and runs as it did. A deny hook on an event whose exit 2 refuses an
    action (pre_tool, pre_commit, user_prompt) fails closed when `specforge` is not on the host's PATH, rendered per host:
    the sh guard on Unix, Claude Code's `onFailure: block`, Codex's plain `commandWindows`, Cursor's plain line on
    Windows; an allow hook, and a stop hook, fail open (ADR 0324). A hook may name the hosts it is installed on (ADR
    0322). While a command is human-attested, install also wires the human guard, which denies a shell call that runs
    one (ADR 0301).
  """
  verify unit "a hook that names its hosts is installed on those only, and is a no-op on another"
  verify unit "a hook whose policy is deny blocks with exit 2 when specforge is not on PATH, and an allow hook's line is the plain command"
  verify unit "a deny hook is fail-closed without sh syntax on Windows: Claude Code's handler carries onFailure block, Codex's carries commandWindows, Cursor's Windows line is the plain command"
  verify unit "a stop or subagent_stop hook never gets the fail-closed line"
  verify unit "no human guard is installed while no command is human-attested"
  verify unit "an uninstall removes every rendered form of a hook line and keeps the user's own"
  verify unit "a project whose extensions contribute nothing to a host declares and runs as it did"
  verify unit "hand-written host files are never read or written by any command"
  verify unit "doctor reports a host file edited since install and an install stale against the project, and nothing for a directory without a marker"
  verify unit "an agent, a skill, a slash command and a hook round-trip through the protocol"
  verify unit "a host name claimed twice is E039 and the later claimant is not rendered"
  verify unit "a contribution the registry refuses is W021 and dropped, and the rest is kept"
  verify unit "a command slash's CLI fallback names the host it was written for"
  verify unit "Claude Code's slash skill leaves effort out, as upstream does"
  verify unit "the registry answers an agent by its id, with the extension that declares it"
  verify unit "a judged rule whose judge is no agent of its extension or a declared peer is W112"
  verify unit "Claude Code gets the agents, skills, slash commands, hooks, statusline and MCP entry of the fixture"
  verify unit "Codex gets agent TOML without an Anthropic model, skills, hooks.json and config.toml"
  verify unit "Cursor gets agents, skills, hooks.json and mcp.json, and reports the statusline and pre_compact as not rendered"
  verify unit "install writes the tree and a second install writes nothing"
  verify unit "install refuses a file it did not write and writes nothing"
  verify unit "install refuses a marked file edited since unless forced"
  verify unit "install removes what no extension contributes any more and keeps a foreign sibling"
  verify unit "install --check exits 1 on drift and names what would change, per host"
  verify unit "uninstall removes exactly what install wrote"
  verify unit "install merges its entries without touching a user's hooks or servers"
  verify unit "install and uninstall never write through a link, even a dangling one, and refuse"
  verify unit "uninstall keeps a user's hook that only mentions specforge, and a settings file it did not create"
  verify unit "a hook command repeats --paths per glob, so a glob holding a comma reaches the hook whole"
  verify unit "a marked file whose line ends became CRLF is not an edited file"
  verify unit "doctor reports an install that wrote only config entries as stale"
  verify unit "install, uninstall and install --check refuse a specforge.json the compile reports as E069 and write nothing"
  verify unit "install refuses a host config of the wrong shape, naming the key, and writes nothing on any host"
  verify unit "uninstall refuses a marked file edited since and names uninstall --force"
  verify unit "uninstall removes only what install created: a config file, a key and a directory the user had stay"
  verify unit "uninstall removes the empty host directories install created, the host's root included"
  verify unit "an install made without a record uninstalls its files and keeps its containers, directories and config files"
  verify unit "a re-install keeps the record, so the uninstall after it still removes the host roots"
  verify unit "the slash commands a prompt is sent are the ones an install writes"
  verify unit "a slash command's hint writes a prompt's free-text tail last"
  verify unit "when reads a name or a list and writes one name as a string, several as a list and none as nothing"
  verify unit "a contribution whose when names several settings is served only while all of them are on"
  verify unit "the SDK's when appends a setting, and a declaration with one is written as a string"
}

behavior run_a_hook "Run a Hook" {
  invariants [extension_isolation]
  category   command
  contract   """
    specforge hook <short>.<id> reads a host's event from stdin, returns the host's no-op answer without loading
    anything when the event, tool class and paths do not match, and otherwise loads only the hook's extension and its
    peers, computes the facts a sandboxed guest cannot read, calls the hook's export within its declared timeout and
    maps the answer to the host's protocol (ADR 0292). A trap, a timeout or a malformed answer applies the hook's
    on_failure policy. A hook that reads no graph and writes nothing is answered over the loaded environment, without a
    compile (ADR 0324). A hook that declares reads_git is told its repository's head (and status), and every path its
    real path (ADR 0320). A hook may name settings of other extensions it reads, and is sent their values (ADR 0323).
    file_changed fires when a file a hook names changes; Claude Code wires it (ADR 0321).
  """
  verify unit "the human guard denies a shell call that runs a human-attested command, from the served descriptors"
  verify unit "the human guard denies a line that names specforge and cannot be read"
  verify unit "a hook that declares reads_git is told its repository's toplevel, git directory and branch, and with reads_git_status the porcelain status"
  verify unit "a hook is told an outside path's real path, and with reads_git its toplevel and whether it is inside a git directory"
  verify unit "history answers the branch and the porcelain status of a repository"
  verify unit "a shell tool's hook is told its command split into commands"
  verify unit "a hook that reads no graph and writes nothing is answered without compiling the project"
  verify unit "a command without state has a byte-equal input"
  verify unit "a hook is given the facts of every path its tool names that a guest cannot read"
  verify unit "a hook that times out or traps applies its failure policy: allow is a no-op with the E028 line, deny is a deny with its text"
  verify unit "pre_commit fires only for git commit, with its message"
  verify unit "a hook's plan is applied through the edit plan step, and a refused plan becomes a notice"
  verify unit "a hook whose when setting is off is a no-op that loads nothing further"
  verify unit "the spec guard denies a raw write of a computed value, naming the field and the tool that writes it"
  verify unit "the spec guard denies a raw write of a human-only value"
  verify unit "the spec guard allows prose and reports the errors an edit adds as context"
  verify unit "the spec guard does not read a person's entity as removed from a file the write leaves unparsable"
  verify unit "the spec guard compares a person's entity as written, not with the values a pass derived"
  verify unit "state a hook patches is what another hook reads, per session"
  verify unit "a state patch that would exceed 64 KiB is refused and reported, and the state is unchanged"
  verify unit "a command reads its state when it declares it and patches it with what it answers"
  verify unit "state is never written, read or pruned through a link, and an oversized document is ignored"
  verify unit "a refused state patch is reported and the files its command's plan wrote stay written"
  verify unit "each host reads its hook stdin into the neutral event and writes the neutral answer as the host expects it"
  verify unit "the SDK's hook builder declares hosts, git facts and other extensions' settings, and a call reads them"
  verify unit "Claude Code wires a file_changed hook on FileChanged, matching the files by name, and reads the changed file and what happened"
  verify unit "a hook is sent the value of another extension's setting it names, and nothing of a setting or extension that is absent"
  verify unit "a file_changed hook fires for the file it names, with what happened to it"
  verify unit "Cursor wires a user_prompt hook on beforeSubmitPrompt and answers a denied prompt with continue false"
  verify unit "Cursor answers a denied tool call with permission deny, its reason to the user and the agent"
  verify unit "Cursor cannot deny after a tool ran: a post_tool deny reaches the agent as context only"
  verify unit "a hook whose event does not match exits before loading anything"
  verify unit "a guard hook denies in the protocol of Claude Code, and in the draft protocols of Codex and Cursor as their goldens have it"
  verify unit "an event over the size limit or not the host's JSON is answered by the hook's failure policy"
  verify unit "specforge hook never exits 2 on its own refusals: an unknown host, event or usage error exits 1"
  verify unit "a writing hook is answered over a compiled project"
  verify unit "the spec guard compiles before it judges"
  verify unit "the spec guard judges the spec files a patch-text tool leaves and ignores a patch of other files"
}
