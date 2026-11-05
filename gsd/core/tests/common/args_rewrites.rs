//! The body lines (0-based) rewritten because a workflow now reads a prompt's typed argument through an `{{arg:}}`
//! placeholder instead of re-parsing the argument text (ADR 0340). `port_workflows` reads them as more
//! `REWRITTEN_SPANS` rows; the port rule's `drops_an_argument_capture` covers an emptied capture line and a dropped
//! `{GSD_WS}` token without a row.

/// `(path, first, last)`, inclusive.
pub const SPANS: &[(&str, usize, usize)] = &[];
