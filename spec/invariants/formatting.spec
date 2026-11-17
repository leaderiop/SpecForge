requirement INVARIANT-048 "Formatting Idempotency" {
  category "Invariant"
  tier     v2
  text     """
    Applying the formatter to an already-formatted file MUST produce identical output.
    Formally: format(format(x)) == format(x) for all valid .spec inputs. Any
    violation of this invariant is a P0 bug.
    It holds for a file with syntax errors too: the forms left unchanged
    are written once, as they were, so a second run reads and keeps the
    same forms (format_with_parse_errors).
  """
}

requirement INVARIANT-006 "Comment Preservation" {
  category "Invariant"
  tier     v2
  text     """
    The formatter MUST NOT lose, relocate, or alter any comments in a .spec file.
    Every comment present in the input MUST appear in the output at the correct
    attachment point. Comment content MUST NOT be modified.
  """
}

requirement INVARIANT-047 "Formatting Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Two semantically identical .spec files MUST produce identical formatted output
    regardless of their original whitespace, indentation, or blank line patterns.
    The formatter MUST be a convergent function: all style variations converge
    to the same canonical form. Parse-error regions are excluded from this
    guarantee: format_with_parse_errors preserves unparseable text verbatim,
    so two files with differently-shaped parse errors MAY produce different output.
    So are lists and statements holding a comment between their tokens,
    which are kept as written so that no comment is lost or moved.
  """
}

requirement INVARIANT-008 "Config Defaults Valid" {
  category "Invariant"
  tier     v2
  text     """
    The default FormatConfig values (indent_width=2, use_tabs=false, max_width=100)
    MUST themselves form a valid configuration. Falling back to defaults MUST
    always produce a usable formatter configuration, never an error state.
  """
}

requirement INVARIANT-019 "Discovery Completeness" {
  category "Invariant"
  tier     v2
  text     """
    When specforge format discovers .spec files under a directory without
    explicit path arguments, it MUST find all .spec files that exist under
    spec_root (as defined in specforge.json) at the time discovery runs.
    Every valid .spec file under spec_root MUST either be formatted or
    explicitly excluded via configuration.
  """
}

requirement INVARIANT-045 "Format Rule Determinism" {
  category "Invariant"
  tier     v2
  text     """
    Given the same input content and the same FormatConfig, the formatter MUST
    produce byte-identical output regardless of execution environment, platform,
    or invocation method (CLI, LSP, stdin). This is stronger than idempotency:
    idempotency guarantees stability across re-application, while determinism
    guarantees stability across independent invocations with identical inputs.
  """
}

requirement INVARIANT-049 "Formatting Semantic Preservation" {
  category "Invariant"
  tier     v2
  text     """
    Formatting MUST NOT alter the semantic content of a .spec file. Formally:
    parse_to_graph(format(spec)) MUST produce an entity graph identical to
    parse_to_graph(spec) — same nodes, same edges, same field values. Only
    whitespace, comment positioning and a value's quotes where its declared
    type writes it the other way (ADR 0147) MAY change. Any formatting operation
    that alters the entity graph is a P0 bug.
  """
}

requirement INVARIANT-046 "Format Rule Application Order" {
  category "Invariant"
  tier     v2
  text     """
    When multiple format rules apply to the same whitespace region,
    rules MUST be applied in a deterministic priority order:
    indent > newline > spacing > alignment > wrapping > comment > import > string.
    Extension-contributed format rules MUST run after all 8 core rules at
    priority level 9 (extension). Multiple extension rules at the same
    priority level MUST be applied in extension load order.
    All compilers MUST produce identical output for the same input and config.
  """
}
