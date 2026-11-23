requirement FEATURE-147 "Dependency and Vulnerability Facts (JavaScript)" {
  category "Feature"
  tier     v2
  text     """
    A JavaScript project's dependencies became facts only from a CycloneDX
    bill of materials someone else had written: no JavaScript pack ran an
    SBOM tool.

    @javascript/deps runs cyclonedx-npm when the project has a
    package-lock.json, through npx --no, which never downloads it, and
    with --package-lock-only, which reads the lockfile alone. It reads the
    bill of materials with the CycloneDX library and answers a dependency
    fact per component, citing the @javascript/source package the bill's
    root names. A bill whose root names no collected package is an error
    naming the report.
  """
}
