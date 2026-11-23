requirement FEATURE-147 "Dependency and Vulnerability Facts (JavaScript)" {
  category "Feature"
  tier     v2
  text     """
    A JavaScript project's dependencies and their known vulnerabilities
    became facts only from a CycloneDX bill of materials or an osv-scanner
    report someone else had written: no JavaScript pack ran either tool.

    @javascript/deps runs cyclonedx-npm when the project has a
    package-lock.json, through npx --no, which never downloads it, and
    with --package-lock-only, which reads the lockfile alone. It reads the
    bill of materials with the CycloneDX library and answers a dependency
    fact per component, citing the @javascript/source package the bill's
    root names. A bill whose root names no collected package is an error
    naming the report.

    A second collector runs osv-scanner on the same lockfile and answers a
    vulnerability fact per OSV record, citing the npm package at or under
    the lockfile's directory. osv-scanner exiting 1 because it found
    vulnerabilities is no failure: the pack reads the report. Every package
    the scan lists that no collected package holds is named in a warning,
    W203 (ADR 0389 D7), never dropped.
  """
}
