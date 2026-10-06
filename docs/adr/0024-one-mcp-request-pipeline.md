# One MCP request pipeline; resources read through the view

**Status:** accepted (2026-10-06). Amends ADR 0004 "MCP tool table" (D4-e) and ADR 0017 D6 and D14's
resource code.

`tools/call`, `resources/read` and `prompts/get` ran the same steps in three functions: read the
request, look the name up, record the invocation, resolve the call target, run the handler, drain
its events, build the reply. `prompts/get` called itself "the prompt-side twin" of `tools/call`. The
copies drifted:

- One failure had four shapes. A missing entity was an `isError` McpError from a tool, a JSON-RPC
  error with the McpError in `data` from a prompt, and a bare `-32602` string from a resource
  ("E003: …" as text from `graph?root=`, "Entity not found" with no E003 from `graph/{id}`), against
  `mcp_structured_error_responses`. A prompt's `file_not_found` on `path` was `-32603`.
- `tools/call` looked an extension tool up before bringing the served project up to date, and an
  unknown name never refreshed it: a tool enabled in `specforge.json` stayed "Unknown tool" until
  another request reloaded the environment. `resources/read` did it in the other order.
- Five of eight core resources read the server state instead of the call target and called the
  emitter themselves, restating the export schema policy. `specforge://graph/{id}` was a Graph
  Protocol 1.0 document without the `schema_ref` a scoped export carries; `?scope=`, the spec's
  parameter, was ignored, and so was every other part of a query the read did not know.
- `path` and `use_cached` were declared by nine schema properties, eleven handler fields kept only
  for a drift test, and that test.
- The two subscription eras wrote "the diagnostics URI changes with the diagnostics, every other with
  the graph" twice, computed the diagnostics delta twice, and `resources/subscribe` accepted URIs the
  server does not serve.
- Operations failed with code strings that MCP mapped with an 11-entry table; 8 slugs and 11
  diagnostic codes fell to `internal_error` (a token budget too small among them), and rename,
  validate and remove special-cased their own.

## Decision

**D1. One pipeline, three adapters.** `specforge_mcp::surface_call::serve::<S: Surface>` runs every
step once; `Tools`, `Resources` and `Prompts` supply lookup, target, handler, events and envelope. The
router guards initialization once for every method that needs a session. The no-project rule
(ADR 0025) is applied in the pipeline, to whichever refusal an outcome holds.

**D2. Extension entries are looked up in an up-to-date project.** A name no core table has brings the
served project up to date before the extension surface table is asked (ADR 0014 D12 for extension
names). `resources/subscribe` does the same before it refuses a URI.

**D3. A resource read refuses like a prompt.** Every refusal is an McpError, sent as the JSON-RPC
error's `data`; McpError gains `uri`, the URI read. One code rule for both
(`McpError::into_rpc_error`): `-32602` when the error names an argument or its code is client input
(`ErrorCode::rpc_code`), else `-32603`. "Unknown tool" and "Unknown prompt" stay plain `-32602`. A
resource that does not exist ("Unknown resource URI", or an entity its URI names) is `-32002` in a
handshake session and `-32602` in a 2026-07-28 request, each revision's server/resources "Error
Handling", with `data.uri`. A no-project refusal names `path` only where the entry takes one: a
resource or a prompt, which refuse a `path`, answer `-32603` without asking for one. With nothing
served, a not-found refusal is the no-project refusal (ADR 0025); an extension resource read with no
project served would be `-32603` (ADR 0017 D14 said `-32602`), a case a real server no longer reaches.

**D4. Graph views are exports.** `graph`, `context`, `brief` (scoped by their template, or by `scope`,
alias `root`) and `graph/{entity_id}` (depth 1) are `ops::export` over `call.view()` under the export
schema policy, byte-equal to `specforge export`. The entity list is `specforge.list`'s function; the
diagnostics are what the view reports. A read answers under the URI it was asked for. A query key the
read does not know, a repeated key, a value that does not parse, `scope` with `root` or a scope on a
templated URI is refused as `invalid_input` naming the key; keys and values are percent-decoded.

**D5. The call target declares its arguments.** `TargetSpec` contributes `path` (by reach) and
`use_cached` (by freshness) to the listed schema and to the field set the drift test reads; handlers
no longer carry them; `init`'s missing `path` is refused by the target.

**D6. One subscription rule.** `subscriptions::Watched::of(uri)` decides what a resource changes with
for both eras; an update's changes are computed once (`Changes`). `resources/subscribe` refuses an
unserved URI as `resources/read` does (not found); `unsubscribe` never refuses.

**D7. Operations fail with a kind.** `specforge_ops::OpErrorKind` is set where a failure is raised;
MCP maps it totally to its McpErrorCode. Diagnostic codes map through one table in ops
(`OpErrorKind::of_diagnostic`). The code stays what the CLI prints.

## Consequences

- User-visible over MCP: a just-enabled extension tool is callable at once; core resource errors
  carry an McpError with `uri`, and a missing resource is `-32002` to a handshake client; a prompt's
  bad `path` is `-32602`; a prompt or resource that needs a project says "start the server in a
  project" instead of asking for a `path`; `graph/{id}` is Graph Protocol 2.0 with `schema_ref`;
  `?scope=` works; a malformed or unknown resource query is refused, no longer ignored; queried reads
  answer under their own URI; `specforge://graph`'s key order is the export's; `use_cached` has one
  description; subscribing to an unserved URI is refused as not found; E062, `export_failed`, the
  R-RES/R-TRUST/R-OPS codes, E033 and failed writes get specific McpError codes (`permission_denied`
  when the OS refused).
- The CLI's output is unchanged.
- An extension call stats the project twice per request (lookup, then target): ~6.5 ms per 1 000
  files.

## Rejected

- **A `match` on the request kind at each step**: three outcome types at every step.
- **`precondition_failed` → `-32602` in the code table**: changes every prompt that needs a project
  to keep one resource case.
- **Refusing `resources/unsubscribe` of an unserved URI**: a client could not drop a subscription to a
  resource its extension stopped serving.
- **Listings folded into the pipeline**: ADR 0017 already makes listing and dispatch one table; the
  listings differ in ways a shared method would have to special-case.
- **Ignoring what a resource query does not read** (partial queries still serving): an agent that
  sends `graph?scop=a` got the full graph and could not tell; the export tool refuses an undeclared
  argument, and the resource over the same function now does too.

## What would reopen it

A fourth request kind that invokes a named entry (completion, an MCP revision's new method) that the
`Surface` trait cannot express without a method most adapters leave empty; or an MCP revision that
changes the not-found code again.
