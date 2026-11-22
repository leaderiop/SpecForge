# osv-scanner reader fixtures

osv-scanner is not installed where these were written, so each fixture is hand-written to the osv-scanner JSON output
(`osv-scanner --format json`: `results[]` of `source` and `packages[]`, each package with its full OSV records and
`groups`) and the OSV schema. The advisories in them are invented (`example.invalid` references, `2099` ids): they
exercise the reader, not a real vulnerability. Phase 11's packs re-record them with the real tool.

| Fixture | Source |
|---------|--------|
| `osv-scanner.json` | hand-written to the osv-scanner JSON output and the OSV schema: one source `/project/Cargo.lock` of type `lockfile`; `smallvec` 1.6.0 (`crates.io`) with a GHSA-shaped record labelled `MODERATE` (`database_specific.severity`), two aliases and an `affected[]` entry for another crate before its own; `time` 0.1.45 with a RustSec-shaped record with no label and no `severity[]` entry |
| `packages.json` | hand-written served facts: the `package` facts a vulnerability collector receives, `pkg:cargo/demo` at the project root |
