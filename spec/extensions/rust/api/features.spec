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
    its path names. A run that read no entry, as when the nightly
    toolchain is missing and the error went to stderr, gives a warning
    that nothing was recorded (W203) instead of looking like a clean run.

    The accepted API is a committed public-api.txt, read beside the run
    (glob **/public-api.txt, files up to 4 MiB) and written with the
    pack's own flags (cargo public-api --color never -ss). Only a crate
    on both sides is judged: each api_item then carries its change
    (added, unchanged, changed, narrowed, removed) against the accepted
    file; without one the facts carry none. A file that does not parse,
    or two files naming one crate, is the collector's error naming
    them; a file the host did not serve,
    or naming no crate the run read, is one warning naming it (W203), so
    an unusable baseline never passes silently. Accepting a break is
    committing the updated file. A narrowing to pub(crate) reads as
    removed, as the tool lists public items only.
  """
}
