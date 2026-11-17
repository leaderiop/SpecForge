requirement INVARIANT-067 "MCP Structured Error Responses" {
  category "Invariant"
  tier     v2
  text     """
    All MCP tools, resources and prompts MUST return structured error objects
    (not plain strings) with error code, message, and the entity_id or file the refusal is about when it
    names one. This
    ensures agents can programmatically handle errors without parsing
    free-form text. A failed tool call is an isError result whose content is
    an McpError; a diagnostic code behind the failure is in its diagnostic,
    not only in the message. A failed prompts/get or resources/read, which
    have no isError result, is a JSON-RPC error whose data is its McpError:
    -32602 when the client can fix it (an argument it named, invalid input,
    an unknown entity or extension, a conflict), -32603 otherwise. A resource
    that does not exist (no entry serves the URI, or the entity it names is
    not in the graph) is -32002 in a handshake session and -32602 in a
    2026-07-28 request, and its data names the URI as uri. A refusal for a
    missing project names the path argument only where the entry takes one.
    A refusal of a call's path names it as the call spelled it.
    An operation's failure kind decides the McpError code. An operation
    states the kind where it raises the failure; no table maps a code to a
    kind, and a package the registry or its trust check refuses is never
    internal_error. A writing tool's call fails only for a failed run, which
    its operation decides: a file it could not read, write or migrate, or a
    change it undid; a finding (a problem in specforge.json it went around,
    a region left unformatted, a change under check) fails its verdict, ok,
    and not the call. An operation states which field of its request a
    refusal is about; a refusal of the project or the environment is about
    none. A depth given without the entity it counts hops from is one
    refusal, invalid input about the entity's field, on every read.
  """
}

requirement INVARIANT-068 "MCP Subscription Cleanup" {
  category "Invariant"
  tier     v2
  text     """
    When an MCP client disconnects, all its subscriptions and listen streams
    MUST be removed, each removal recorded. Subscriptions belong to the
    connection: no request parameter names another client. No orphan
    subscriptions may remain after client disconnect. This prevents resource
    leaks and ensures notification delivery targets only active clients.
  """
}

requirement INVARIANT-069 "MCP Tool Idempotency" {
  category "Invariant"
  tier     v2
  text     """
    Read-only MCP tools (core, navigation, and read-only project management:
    extensions, providers, doctor) and all MCP
    prompts MUST be idempotent: repeated calls with the same parameters MUST
    return the same result if the graph has not changed between calls. This
    guarantees agents can safely retry read operations and prompt invocations.
  """
}

requirement INVARIANT-066 "MCP Served Project Consistency" {
  category "Invariant"
  tier     v2
  text     """
    The project an MCP server serves is replaced whole or not at all, and
    every request that reads it (tools/call, resources/read, prompts/get and
    the list methods) first brings it up to date with the files on disk
    (bring_session_up_to_date):
    an update for changed sources, an environment reload with its extension
    tools and resources for a changed specforge.json, specforge.lock or
    extension module. Subscribed clients learn what changed. A call whose
    path names another project, one nested in the served project, acts on
    that project only, opened for the call and brought up to date after it
    writes as the served project is, and the server keeps serving its own
    without reloading it. A path an MCP request names resolves inside the
    served project (while none is served, inside the server's root): a
    relative path is read against that directory, never the process's, and
    a path that leaves it, by .., by an absolute name or by a symlink, is
    refused as invalid_input on its argument (a path behind such a symlink
    is outside whether or not the file exists), whatever tool, prompt or
    resource takes it and in either revision. A file or directory a tool
    argument names (format's paths, render's out_dir, outline's file)
    resolves inside the root of the project the call acts on, and no
    mutation reads or writes through a symlink that leaves it (init's
    files, the lock and the installed modules included), and a file the
    project keeps at its root (the inference manifest, the fact stores,
    the test report, the schema cache) is read only from inside it. A source of the served
    project, the recorded report and the schema cache are also read only from a file that has
    one name: a hard link to a file outside is another name for it. So is
    the format configuration (.specforgefmt.toml) a call reads (format's, a
    typed edit's layout, init's starter), and format reads each file it
    formats only when it has one name. A typed
    edit is validated over the sources the served project read, and a
    migration reads each source, and compiles the project before and after
    it, through the call's bound, so neither a
    preview nor a refusal carries what a refused source holds. init creates
    its project inside the server's root and never inside the served
    project. The CLI is not bounded: its user is already at the file system.
    A call
    that names a tool, a resource or a prompt looks it up in the project as
    it is on disk: an extension enabled since the last request is found by
    the next one, whether the request is a call, a read, a listing or a
    subscription (one freshness decision, owned by the request pipeline). Only a
    project (a directory holding specforge.json) is ever
    served: the server serves the nearest one at or above its root (the
    client's projectRoot, else the path it was started with), and nothing
    when there is none; while none is served, every request that reads
    what is served looks there again first, so a project created there
    since is served by the next such request. In a handshake session, a call whose path names a project
    while none is served serves that project, and init serves the project
    it creates; a stateless request (MCP 2026-07-28) never changes what is
    served: its path is read for that call only and init serves nothing. A
    path to a directory that is no project is read for that call only and
    never served.
    With no project served and none named, what a call gets is declared by
    the entry's target: an entry that reads only the project view (the read
    views, export and render, the navigation reads of the graph, every core
    prompt and resource) answers over the empty session, and one of its
    reads that names a file or an entity is the no-project refusal
    (precondition_failed), never not-found; an entry that acts on the
    project on disk (a tool that takes a path, a management tool, a read of
    the inference manifest, every extension tool and resource)
    is refused as no project before its arguments are read.
  """
}

requirement INVARIANT-065 "MCP Event Record" {
  category "Invariant"
  tier     v2
  text     """
    What an MCP server records it did (the events of spec/events/mcp.spec and
    spec/events/surface-contributions.spec, and the extension_added and
    project_initialized a mutation produces) is one typed value per event,
    its payload the spec's field for field, recorded through the server's
    one event sink. Events never leave the process: a server keeps none of
    them, so what it holds does not grow with the requests it answered; a
    test gives the server a sink that keeps them, to observe it. A sink that
    keeps an event stamps it with the time it was recorded (timestamp).
  """
}
