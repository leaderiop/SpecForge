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
    generated-by marker holding its source and digest, a config entry it owns is the one marked `# specforge-install` (a hook) or named specforge (the MCP server). It never follows a link.
    It refuses to overwrite an unmarked file (foreign_file) or a marked file edited since (edited_file, unless
    forced), removes what no extension contributes any more and reports what a host cannot take. A project whose
    extensions contribute nothing to a host declares and runs as it did.
  """
  verify unit "a project whose extensions contribute nothing to a host declares and runs as it did"
  verify unit "hand-written host files are never read or written by any command"
  verify unit "doctor reports a host file edited since install and an install stale against the project, and nothing for a directory without a marker"
  verify unit "an agent, a skill, a slash command and a hook round-trip through the protocol"
  verify unit "a host name claimed twice is E039 and the later claimant is not rendered"
  verify unit "a contribution the registry refuses is W021 and dropped, and the rest is kept"
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
}

behavior run_a_hook "Run a Hook" {
  invariants [extension_isolation]
  category   command
  contract   """
    specforge hook <short>.<id> reads a host's event from stdin, returns the host's no-op answer without loading
    anything when the event, tool class and paths do not match, and otherwise loads only the hook's extension and its
    peers, computes the facts a sandboxed guest cannot read, calls the hook's export within its declared timeout and
    maps the answer to the host's protocol (ADR 0292). A trap, a timeout or a malformed answer applies the hook's
    on_failure policy.
  """
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
  verify unit "a hook whose event does not match exits before loading anything"
  verify unit "a guard hook denies in the protocol of Claude Code, and in the draft protocols of Codex and Cursor as their goldens have it"
  verify unit "an event over the size limit or not the host's JSON is answered by the hook's failure policy"
  verify unit "the spec guard judges the spec files a patch-text tool leaves and ignores a patch of other files"
}
