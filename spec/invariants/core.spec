requirement INVARIANT-131 "Spec Root Singleton" {
  category "Invariant"
  tier     v2
  text     """
    Exactly one project root MUST exist per project: the directory
    holding its specforge.json, located from any path inside the
    project by locate_project_root. The specforge.json file declares
    project identity (name, version), installed extensions, and
    provider configurations; no other file marks or configures a
    project.
  """
}

requirement INVARIANT-058 "Init Config Validity" {
  category "Invariant"
  tier     v2
  text     """
    Any specforge.json produced by specforge init or modified by
    specforge add MUST be syntactically valid JSON and semantically
    valid per the SpecForgeJsonConfig schema. The generated config
    MUST be parseable by the compiler pipeline without errors. This
    invariant ensures the seconds-to-value principle: init followed
    by check MUST never fail due to malformed configuration.
  """
}

requirement INVARIANT-082 "Multi-Error Collection" {
  category "Invariant"
  tier     v2
  text     """
    The compiler MUST collect and report all diagnostics found during a
    compilation pass. It MUST NOT halt on the first error. Every error,
    warning, and info diagnostic MUST be emitted to the user.
  """
}

requirement INVARIANT-132 "String Interning Consistency" {
  category "Invariant"
  tier     v2
  text     """
    All interned strings MUST resolve to the same symbol for the same input
    within a compilation session. Two equal strings MUST produce the same
    interned key. Comparison by interned key MUST be equivalent to
    comparison by string value.
  """
}

requirement INVARIANT-056 "Import DAG" {
  category "Invariant"
  tier     v2
  text     """
    The use import graph SHOULD form a directed acyclic graph (DAG).
    References resolve across the whole project without use, so a cycle
    changes nothing: the compiler MUST detect circular imports and report
    each with a W113 warning that names the cycle participants, in the
    same order on every run. Files in a cycle are still compiled.
  """
}

requirement INVARIANT-025 "Entity ID Uniqueness" {
  category "Invariant"
  tier     v2
  text     """
    Every entity name MUST be unique within its scope across all .spec files
    in a project: a scope is the entity its kind's scope field names, or the
    project. Two entities of one kind with the same name in one scope are
    forbidden: the compiler MUST reject the duplicate with an E002 diagnostic
    that names both declaration sites. Two of different kinds are a W060. An
    entity's ID is its scope's path and its name.
  """
}

requirement INVARIANT-054 "Graph Traversal Integrity" {
  category "Invariant"
  tier     v2
  text     """
    Graph traversal operations (trace, subgraph extraction, delta
    computation) MUST produce complete and deterministic results. Every
    reachable node along a traversal path MUST be included. Traversal
    order MUST be deterministic for identical graph inputs. A traversal
    reads only the edges of the entities it reaches, never every edge of
    the graph for each of them.
  """
}

requirement INVARIANT-057 "Incremental Correctness" {
  category "Invariant"
  tier     v2
  text     """
    After an incremental recompilation triggered by a file change, the
    in-memory graph MUST be identical to the graph produced by a full
    cold rebuild of the same source files. No stale nodes or edges
    MUST remain from the previous compilation. What a compile reports is
    a function of its environment and its sources (their paths and
    texts): no file it does not hold changes it, so a file discovery
    does not find (excluded, under a skipped directory, outside the spec
    root) is no input. After every update that runs the checks, a
    project session MUST report exactly what a fresh compile of the same
    sources reports, in the same order; an editor update that skips the
    checks while a file does not parse reports no check diagnostic until
    the next update that runs them.
  """
}

requirement INVARIANT-052 "Graph Delta Determinism" {
  category "Invariant"
  tier     v2
  text     """
    Given identical previous and current graph states, compute_graph_delta
    MUST produce an identical GraphDelta. All arrays in GraphDelta
    (added_nodes, removed_nodes, modified_nodes, added_edges, removed_edges)
    MUST be sorted by EntityId.raw (lexicographic). The ordering MUST NOT
    depend on hash-map iteration order, filesystem order, or any
    non-deterministic source.
  """
}

requirement INVARIANT-053 "Graph Schema Completeness" {
  category "Invariant"
  tier     v2
  text     """
    The schema section of any Graph Protocol export MUST include every
    entity kind and edge type registered in the KindRegistry and edge type
    set at the time of export. No registered kind or edge type MUST be
    omitted. The schema MUST accurately reflect the testable, singleton,
    and field definitions for each kind.
  """
}

requirement INVARIANT-125 "Schema Version Backward Compatibility" {
  category "Invariant"
  tier     v2
  text     """
    The previous major version of the Graph Protocol schema MUST remain
    readable by the current compiler. Breaking changes to the schema MUST
    only occur on major version increments. The compiler MUST be capable of
    loading and interpreting Graph Protocol JSON produced by any version
    within the same major version range.
  """
}

requirement INVARIANT-148 "Watch Mode Response Latency" {
  category "Invariant"
  tier     v2
  text     """
    File-change-to-diagnostics in watch mode MUST complete within 100ms
    for single-file changes.
  """
}

requirement INVARIANT-141 "Token Budget Subgraph Consistency" {
  category "Invariant"
  tier     v2
  text     """
    When enforce_token_budget truncates entities from export output, the
    remaining subgraph MUST NOT contain dangling edges referencing truncated
    nodes. Truncated entity IDs MUST be listed in the truncated_entities
    response field.
  """
}

requirement INVARIANT-116 "Query File Grammar Consistency" {
  category "Invariant"
  tier     v2
  text     """
    All .scm query files (highlights.scm, folds.scm, indents.scm) MUST
    remain valid and consistent with the tree-sitter grammar after any
    grammar change. Node names, capture names, and pattern structures in
    query files MUST reference grammar rules that exist in the current
    grammar version. A grammar change that adds, removes, or renames a
    rule MUST trigger review of all .scm files for broken references.
  """
}

requirement INVARIANT-011 "CST Vocabulary Grammar Consistency" {
  category "Invariant"
  tier     v2
  text     """
    The node kinds and field names the parser and the formatter read from
    tree-sitter's concrete syntax tree MUST be named once, by the grammar
    crate (tree_sitter_specforge::kind and tree_sitter_specforge::field),
    and MUST be exactly the named node kinds and field names of the
    current grammar: a grammar change that adds, removes or renames a rule
    or a field fails a test until the vocabulary follows it (ADR 0038).
  """
}

requirement INVARIANT-021 "Dry-Run Side-Effect Freedom" {
  category "Invariant"
  tier     v2
  text     """
    Any command invoked with --dry-run or --check MUST NOT modify any files
    on disk. These flags guarantee read-only execution. If a code path
    reachable from a dry-run context writes to the filesystem, it is a P0
    bug. This applies to specforge migrate --dry-run, specforge add
    --dry-run, specforge remove --dry-run, specforge format --check,
    specforge format --diff, and any future commands that support dry-run mode.
    Over MCP a preview flag is read by its type: dry_run, check or diff sent as
    the string "true" is a preview.
  """
}

requirement INVARIANT-020 "Document Project Paths" {
  category "Invariant"
  tier     v2
  text     """
    A field of a surface document that names a file or directory an
    operation opened by path or wrote MUST be relative to the root of the
    project the call acted on, with / between components; a path outside
    that root (only the CLI can name one) is absolute and canonical. A
    document's anchor directory (init's project_root, watch's spec_root) is
    absolute. A location the graph declares (a span, a navigation result,
    an export node, rename's plan) is named by its source key, which is its
    project path. The CLI and MCP print the same bytes for one run,
    whatever the working directory or the spelling of the root.
  """
}

requirement INVARIANT-130 "Source Span Completeness" {
  category "Invariant"
  tier     v2
  text     """
    Every AST node produced by the parser MUST carry a valid SourceSpan
    with start and end positions that accurately reflect the original
    source text. No AST node MUST have a zero-length span unless it
    represents a synthetic node inserted by error recovery.
  """
}

requirement INVARIANT-144 "Typed Edit Canonical Form" {
  category "Invariant"
  tier     v2
  text     """
    A value a typed edit writes MUST be in the form its field's declaration
    decides, so a typed edit never produces W159 or W020, and never writes a
    field that holds what its file's default holds, as the graph stores both (so no W161 restates it and no
    default it reaches is left dead). Every block a typed edit
    touches MUST be format-clean afterwards, laid out with the project's
    format configuration, its multi-line values included, and no byte outside
    the touched blocks changes; a file whose every line break is CR LF is
    written so (ADR 0266).
  """
}

requirement INVARIANT-143 "Typed Edit Atomicity" {
  category "Invariant"
  tier     v2
  text     """
    A typed edit MUST NOT be written when the edited project reports an error
    the project did not report before (E080), nor when its graph changes what
    its edits do not declare (undeclared_change). Its files are written as one
    project change: every file or none, and a file changed since the project
    was compiled refuses the whole plan.
  """
}
