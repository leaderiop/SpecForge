//! Judged rules (ADR 0290): a rule whose check is `judged` holds while a fresh verdict from its judge says so. The
//! declaration is checked here, once, at registration; evaluating it over a project is the project crate's.

mod rule;
mod selector;

pub use rule::{CITED, CITED_TESTS, JudgedRule, JudgedTargetRule, PROMPT_JUDGE};
pub(crate) use rule::{resolve, shape};
pub use selector::{Chain, Head, Selector, Step};
