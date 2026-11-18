requirement INVARIANT-060 "LSP Response Latency" {
  category "Invariant"
  tier     v2
  text     """
    Diagnostic updates MUST appear within 100ms of the user stopping
    typing. Completion, hover, go-to-definition, and formatting responses
    MUST return within 200ms for projects under 1000 entities.
  """
}

requirement INVARIANT-059 "LSP Extension Reload Consistency" {
  category "Invariant"
  tier     v2
  text     """
    When extensions are added or removed while the LSP server is running,
    KindRegistry, FieldRegistry, and the semantic token legend MUST update
    atomically. No LSP request served between the start and end of the
    update MUST observe a partially-updated registry state. The LSP MUST
    NOT serve stale semantic tokens, completions, or hover information
    for entity kinds that were added or removed.
  """
}

requirement INVARIANT-123 "Rename Atomicity" {
  category "Invariant"
  tier     v2
  text     """
    A rename operation MUST update all files atomically — either all files
    are updated or none are. Partial updates MUST NOT persist.
  """
}

requirement INVARIANT-063 "LSP TextEdit Non-Overlapping" {
  category "Invariant"
  tier     v2
  text     """
    TextEdit operations returned in a single LSP response MUST NOT have
    overlapping ranges. The LSP specification requires non-overlapping edits;
    overlapping edits cause undefined client behavior.
  """
}

requirement INVARIANT-061 "LSP State Concurrency Safety" {
  category "Invariant"
  tier     v2
  text     """
    The LSP's shared state (open documents and the compiled graph) MUST
    stay consistent under concurrent requests: readers MUST NOT block each
    other, a reader MUST never observe a half-applied update, interleaved
    reads and writes MUST NOT deadlock, and writes to different documents
    MUST NOT interfere.
  """
}

requirement INVARIANT-064 "LSP UTF-16 Positions" {
  category "Invariant"
  tier     v2
  text     """
    Every position the LSP receives or returns MUST count columns in UTF-16
    code units, as the Language Server Protocol requires, so non-ASCII text
    before the cursor never shifts the word, range or edit it resolves to.
    One line index per text converts byte offsets and UTF-16 positions,
    both ways; a column past its line's end, whatever its value, is that
    end; nothing else in the LSP converts them (ADR 0023). A span of
    the graph or of a diagnostic is a position in the text the project was
    compiled from, so it converts against that text, never against the
    buffer typed since nor the disk now, and is a range only of an editor
    text (the open buffer, else the file on disk) that is that text: in a
    file typed since, changed on disk since, or of which the compile holds
    no text, its location, symbol or definition is left out and a fix or a
    rename that would need it is refused whole, never byte columns passed
    off as UTF-16. A diagnostics publish, labelled with the version of the
    text the project was compiled from, places its diagnostics in that
    text. While the document a code-action request names is not
    that text, no action is listed, whether the client resolves code
    actions or not, and none is planned.
  """
}

requirement INVARIANT-062 "The LSP Survives Its Faults" {
  category "Invariant"
  tier     v2
  text     """
    A panic in a request or notification handler is a fault of the server,
    not the end of the editor's session: a request answers -32603
    InternalError, a notification changes nothing more, the client is told
    what failed in a window/logMessage error, and the server keeps serving.
    A recompile's panic is reported the same way (ADR 0043). No input a
    client can send ends the process: a position, a range or a buffer the
    server cannot use is clamped or refused.
  """
}

requirement INVARIANT-012 "One Entity Under the Cursor" {
  category "Invariant"
  tier     v2
  text     """
    Every LSP request about the entity under a cursor (hover,
    go-to-definition, references, rename) MUST resolve the same entity: the
    declaration or reference token under the cursor as navigation reads it,
    else an identifier at a reference position (an entity header's name, a
    value or list item in the entity's own body of a field not typed as
    enum, boolean, integer, string, string list or block, a use binding's
    imported name) that names an entity. A word in a string (one spanning lines included) or a comment, a kind keyword, a field name
    and a value of a non-reference field name no entity, but a quoted
    reference's ID, which is the reference token navigation reads (ADR 0218); a scheme ref ID
    (gh.issue:42) is one token. The structure around the
    cursor is read from the document's text, never from the graph, which
    lags the text while the user types (ADR 0023). While the document is not
    the text the project was compiled from (an edit waits for its compile),
    the token is never looked up by its position in the compiled text: the
    cursor names what its own word names at a reference position. Hover,
    definition and references then answer for that entity, with no
    location in a file whose editor text is not the compiled text
    (lsp_utf16_positions).
  """
}
