//! Ported from gsd-core v1.19.0 hooks/lib/injection-patterns.js and hooks/gsd-read-injection-scanner.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The backtracker `regex.rs` ran before plan 01 (G2), kept as the oracle of `the_matcher_agrees_with_the_oracle`: a
//! closure per step, restarted at every position. It is quadratic on `MD-LINK-USERINFO`, so it only sees short inputs.

use crate::hooks::injection::regex::{Node, Regex};

fn go(n: &Node, s: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
    match n {
        Node::Char(_) | Node::Space | Node::Class { .. } => {
            i < s.len() && Regex::one(n, s[i]) == Some(true) && k(i + 1)
        }
        Node::Start => i == 0 && k(i),
        Node::Seq(v) => seq(v, s, i, k),
        Node::Alt(v) => {
            for a in v {
                if go(a, s, i, k) {
                    return true;
                }
            }
            false
        }
        Node::Rep { node, min, max } => {
            // A run of one-character atoms is counted, not recursed into: a megabyte of spaces is input.
            if Regex::one(node, ' ').is_some() {
                let mut run = 0;
                while run < *max && i + run < s.len() && Regex::one(node, s[i + run]) == Some(true)
                {
                    run += 1;
                }
                return (*min..=run).rev().any(|take| k(i + take));
            }
            rep(node, (*min, *max), 0, s, i, k)
        }
        Node::Look { neg, node } => {
            let hit = go(node, s, i, &mut |_| true);
            hit != *neg && k(i)
        }
    }
}

fn seq(v: &[Node], s: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
    match v.split_first() {
        None => k(i),
        Some((head, tail)) => go(head, s, i, &mut |j| seq(tail, s, j, k)),
    }
}

fn rep(
    n: &Node,
    (min, max): (usize, usize),
    count: usize,
    s: &[char],
    i: usize,
    k: &mut dyn FnMut(usize) -> bool,
) -> bool {
    if count < max
        && go(n, s, i, &mut |j| {
            (j != i || count < min) && rep(n, (min, max), count + 1, s, j, k)
        })
    {
        return true;
    }
    count >= min && k(i)
}

/// The first match of `node` in `text`: where it starts and ends, in characters.
pub(super) fn find(node: &Node, text: &[char]) -> Option<(usize, usize)> {
    (0..=text.len()).find_map(|start| {
        let mut end = None;
        go(node, text, start, &mut |e| {
            end = Some(e);
            true
        })
        .then(|| (start, end.unwrap_or(start)))
    })
}
