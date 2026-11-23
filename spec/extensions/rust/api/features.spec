requirement FEATURE-150 "API Snapshot Facts (Rust)" {
  category "Feature"
  tier     v2
  text     """
    A Rust crate's public API became facts only from a cargo public-api
    snapshot someone else had written: no Rust pack ran the tool.

    @rust/api runs cargo public-api when the project has a Cargo.toml,
    omitting blanket and auto-trait impls. The tool prints to standard
    output only, which the host captures for the pack. It reads one
    package per run: the root package, unless the caller's filter names
    another, which the host passes as --package. It builds the crate's
    rustdoc JSON with a nightly toolchain it finds by itself. The pack
    reads the text with the API library and answers an api_item fact per
    public entry, citing the one @rust/source item of the library crate
    its path names.
  """
}
