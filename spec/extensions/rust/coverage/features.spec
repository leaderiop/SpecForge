requirement FEATURE-143 "LCOV Coverage Facts (Rust)" {
  category "Feature"
  tier     v2
  text     """
    A Rust project's coverage became facts only from an LCOV file someone
    else had written: no Rust pack ran a coverage tool, and the LCOV reader
    failed the whole report when one file lay outside the project.

    @rust/coverage runs cargo-llvm-cov over nextest for the workspace, with
    failing tests still writing the report, reads its LCOV with the LCOV
    library and answers a coverage fact per @rust/source item its lines
    reach, one per file for what no item holds. A file outside the project,
    a path dependency beside the workspace, is skipped and counted, and the
    pack names the report and the skipped paths in one warning, W203
    (ADR 0389 D7).
  """
}
