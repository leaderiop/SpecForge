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
    generated-by marker holding its source and digest, a config entry it owns is the one whose command is specforge.
    It refuses to overwrite an unmarked file (foreign_file) or a marked file edited since (edited_file, unless
    forced), removes what no extension contributes any more and reports what a host cannot take. A project whose
    extensions contribute nothing to a host declares and runs as it did.
  """
  verify unit "a project whose extensions contribute nothing to a host declares and runs as it did"
  verify unit "hand-written host files are never read or written by any command"
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
}
