requirement FEATURE-148 "Dependency and Vulnerability Facts (Rust)" {
  category "Feature"
  tier     v2
  text     """
    A Rust project's dependencies and their known vulnerabilities became
    facts only from a CycloneDX bill of materials or an osv-scanner report
    someone else had written: no Rust pack ran either tool.

    @rust/deps runs cargo-cyclonedx when the project has a Cargo.toml.
    cargo-cyclonedx writes one bill of materials beside each workspace
    member's manifest and takes no output path, so the collector's report
    is the glob **/rust-sbom.cdx.json, read fresh after the run. Every
    bill the glob finds becomes a dependency fact per component, citing
    the @rust/cargo package its root names; a bill whose root names no
    collected package is an error naming that report.

    A second collector runs osv-scanner on Cargo.lock and answers a
    vulnerability fact per OSV record, citing the cargo packages at or
    under the lockfile's directory. osv-scanner exiting 1 because it found
    vulnerabilities is no failure: the pack reads the report. Every package
    the scan lists that no collected package holds is named in a warning,
    W203 (ADR 0389 D7), never dropped.
  """
}
