requirement FEATURE-149 "API Snapshot Facts (JavaScript)" {
  category "Feature"
  tier     v2
  text     """
    A JavaScript package's public API became facts only from an
    api-extractor doc model someone else had written: no JavaScript pack
    ran api-extractor.

    @javascript/api runs api-extractor through npx --no, which never
    downloads it, when the project has a tsconfig.json. The pack owns the
    config file the host writes for the run: its entry is the built
    dist/index.d.ts, so the project builds its declarations first, and it
    writes only the doc model, to the report path; no API report, rollup
    or metadata file lands in the project. The pack reads the doc model
    with the API library and answers an api_item fact per member, citing
    the one @javascript/source item of the package with that name path.

    The accepted API is the report the project commits for review,
    read beside the run (files up to 4 MiB): api-extractor's
    etc/<package>.api.md (glob **/etc/*.api.md) or rush's
    common/reviews/api/<package>.api.md. api-extractor's temp/ copy,
    the fresh run's own, and the alpha, beta and public report variants
    are never read. Only the package the run read is judged: each
    api_item then carries its change (added, unchanged, changed,
    narrowed, removed) against its report; without one the facts carry
    none. A report of the package that does not parse, or two reports
    of the package, is the collector's error naming them; a file the
    host did not serve, with no report title, or of another package,
    broken or not, is one warning naming it (W203) and the collect goes
    on.
    Accepting a break is committing the updated report, written by the
    project's own api-extractor run --local with the report enabled and
    the pack's entry.
  """
}
