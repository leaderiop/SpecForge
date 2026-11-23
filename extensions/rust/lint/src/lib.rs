//! @rust/lint — runs cargo clippy and records its diagnostics as `@code/core`'s `static_finding` facts (ADR 0389).
//!
//! clippy writes no SARIF: [`clippy`] converts cargo's JSON messages into the SARIF reader's results (ADR 0389 D8).

// The collector reads it from the next commit on.
#[allow(dead_code)]
mod clippy;

#[cfg(test)]
mod tests;
