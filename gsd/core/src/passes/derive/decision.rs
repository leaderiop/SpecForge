//! A decision's `in_force`: false once a decision supersedes it with force. It is the answer of
//! `passes::plans::superseded`, the set gsd/E310, W317 and W506 leave out, so a prompt section that serves decisions
//! (which no selector can filter on a computed field) shows the same answer the passes read.

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::{of_kind, superseded};

pub fn values(input: &PassInput) -> Vec<PassValue> {
    let superseded = superseded(input);
    of_kind(input, "decision")
        .map(|d| value(&d.id, "in_force", !superseded.contains(d.id.as_str())))
        .collect()
}

#[cfg(test)]
mod tests;
