# @javascript/deps fixtures

| Fixture | Source |
|---------|--------|
| `cyclonedx-npm-7.0.1.cdx.json` | recorded by the Phase 11 research with cyclonedx-npm 7.0.1 (npm 11.20.0): `cyclonedx-npm --output-format JSON --spec-version 1.6 --package-lock-only` over a project whose root package is `board` 1.0.0 (`pkg:npm/board@1.0.0`); committed as recorded. It names no absolute path |
| `packages.json` | hand-written served facts: the `package` facts the collectors receive, `pkg:npm/board` at the project root and `pkg:npm/web` at `web` (the npm package the OSV reader's recorded `web/package-lock.json` belongs to) |
