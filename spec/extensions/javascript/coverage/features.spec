requirement FEATURE-144 "LCOV Coverage Facts (JavaScript)" {
  category "Feature"
  tier     v2
  text     """
    A JavaScript project's coverage became facts only from an LCOV file
    someone else had written: no JavaScript pack ran a coverage tool.

    @javascript/coverage runs c8 around the project's own npm test, so the
    project's test command decides what runs, through npx --no, which never
    downloads c8. It reads c8's LCOV with the LCOV library and answers a
    coverage fact per @javascript/source item its lines reach, one per file
    for what no item holds. A file outside the project is skipped and
    counted, and the pack names the report and the skipped paths in one
    warning, W203 (ADR 0389 D7).
  """
}
