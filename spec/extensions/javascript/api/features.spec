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
  """
}
