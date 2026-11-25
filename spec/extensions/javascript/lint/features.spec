requirement FEATURE-145 "SARIF Lint Facts (JavaScript)" {
  category "Feature"
  tier     v2
  text     """
    A JavaScript project's lint findings became facts only from a SARIF log
    someone else had written: no JavaScript pack ran a linter.

    @javascript/lint runs ESLint with the @microsoft/eslint-formatter-sarif
    formatter when the project has an eslint.config file, through npx --no,
    which never downloads ESLint, so the project's own config decides the
    rules. It reads the SARIF with the SARIF library and answers a
    static_finding fact per result, citing the innermost
    @javascript/source item holding its line, else the package holding its
    file. A result in a file outside the project is skipped and counted,
    and the pack names the report and the skipped paths in one warning,
    W203 (ADR 0389 D7).
  """
}
