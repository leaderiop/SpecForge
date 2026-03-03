// Formatting invariants — guarantees the formatter must always uphold

invariant formatting_idempotency "Formatting Idempotency" {
  guarantee """
    Applying the formatter to an already-formatted file MUST produce identical output.
    Formally: format(format(x)) == format(x) for all valid .spec inputs. Any
    violation of this invariant is a P0 bug.
  """
  enforced_by [apply_format_rules, maintain_format_idempotency]
  risk high

  verify property "formatting an already-formatted file produces identical output"
  verify property "random valid .spec files satisfy format(format(x)) == format(x)"
}

invariant comment_preservation "Comment Preservation" {
  guarantee """
    The formatter MUST NOT lose, relocate, or alter any comments in a .spec file.
    Every comment present in the input MUST appear in the output at the correct
    attachment point. Comment content MUST NOT be modified.
  """
  enforced_by [preserve_comments, apply_format_rules]
  risk high

  verify property "every comment in input appears in formatted output"
  verify unit "trailing comments remain attached to their preceding node"
  verify unit "leading comments remain attached to their following node"
}

invariant formatting_consistency "Formatting Consistency" {
  guarantee """
    Two semantically identical .spec files MUST produce identical formatted output
    regardless of their original whitespace, indentation, or blank line patterns.
    The formatter MUST be a convergent function: all style variations converge
    to the same canonical form.
  """
  enforced_by [apply_format_rules]
  risk medium

  verify property "two files differing only in whitespace produce identical formatted output"
  verify unit "tab-indented and space-indented inputs produce the same output"
}
