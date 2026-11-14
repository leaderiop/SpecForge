//! The computed values of a risk (`spec/governance.md`): its `rpn` and `post_rpn`, from its ratings.
//!
//! One seam rates a risk, [`rate`]: the product of severity, occurrence and detection, and of the post ratings, a post
//! member left out keeping its pre rating. A risk whose rating is missing or outside 1 to 10 gets no value: the kernel
//! reports it (a required field, E061), and no number is guessed.

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::{block, of_kind, text};

/// A risk's three ratings, each 1 to 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sod {
    pub severity: i64,
    pub occurrence: i64,
    pub detection: i64,
}

impl Sod {
    fn product(self) -> i64 {
        self.severity * self.occurrence * self.detection
    }
}

/// The ratings of a risk's `post` block, each `None` when left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Post {
    pub severity: Option<i64>,
    pub occurrence: Option<i64>,
    pub detection: Option<i64>,
}

/// What a risk's ratings compute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rating {
    /// severity × occurrence × detection.
    pub rpn: i64,
    /// The same product over the post ratings: `rpn` without a post block.
    pub post_rpn: i64,
}

/// Rates a risk: pure and total over ratings already read in range.
pub fn rate(pre: Sod, post: Post) -> Rating {
    let after = Sod {
        severity: post.severity.unwrap_or(pre.severity),
        occurrence: post.occurrence.unwrap_or(pre.occurrence),
        detection: post.detection.unwrap_or(pre.detection),
    };
    Rating {
        rpn: pre.product(),
        post_rpn: after.product(),
    }
}

/// A rating as written: an integer from 1 to 10.
fn rating(written: &str) -> Option<i64> {
    written
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|r| (1..=10).contains(r))
}

/// A risk's ratings, `None` when a required one is missing or any written one is outside 1 to 10.
fn read(risk: &PassEntity) -> Option<(Sod, Post)> {
    let pre = |name: &str| text(risk, name).and_then(rating);
    let sod = Sod {
        severity: pre("severity")?,
        occurrence: pre("occurrence")?,
        detection: pre("detection")?,
    };
    let members = block(risk, "post");
    // `Some(None)`: left out; `None`: written and refused.
    let member = |name: &str| match members.get(name) {
        None => Some(None),
        Some(written) => rating(written).map(Some),
    };
    let post = Post {
        severity: member("severity")?,
        occurrence: member("occurrence")?,
        detection: member("detection")?,
    };
    Some((sod, post))
}

/// `rpn` and `post_rpn` of each risk whose ratings are read.
pub fn values(input: &PassInput) -> Vec<PassValue> {
    of_kind(input, "risk")
        .filter_map(|risk| Some((risk, read(risk)?)))
        .flat_map(|(risk, (pre, post))| {
            let rated = rate(pre, post);
            [
                value(&risk.id, "rpn", rated.rpn),
                value(&risk.id, "post_rpn", rated.post_rpn),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests;
