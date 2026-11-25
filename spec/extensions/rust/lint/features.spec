requirement FEATURE-146 "Clippy Lint Facts (Rust)" {
  category "Feature"
  tier     v2
  text     """
    A Rust project's clippy diagnostics became facts only from a SARIF log
    someone else had converted: no Rust pack ran clippy, and clippy writes
    no SARIF.

    @rust/lint runs cargo clippy over the whole workspace and all targets
    with cargo's JSON messages, and the host captures its standard output.
    The pack converts each diagnostic's primary span into a SARIF result,
    keeps one of the copies --all-targets writes for a library checked as
    lib and as lib test, and answers a static_finding fact per diagnostic,
    citing the innermost @rust/source item holding its line, else the
    package holding its file. A cut-off capture is an error: a line cut
    inside names its number, and a capture with no build-finished record,
    which cargo writes last on every run that ends, is a run cut off.
    A span in a file outside the project is skipped and counted, and the
    pack names the capture and the skipped paths in one warning, W203
    (ADR 0389 D7). A span naming no file, such as rustc's <std macros>,
    is skipped and counted the same way in a warning of its own, never
    minted at a file no one can open.
  """
}
