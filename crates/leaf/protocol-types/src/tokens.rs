//! The one token estimate the host and its guests share: a heuristic, not a tokenizer. Each word-like run counts one
//! token and each JSON structural character half. A budget in tokens, a context route's or `--max-tokens`', is in
//! these units, so a guest that declares one and the host that fits it count alike.

/// The token cost a `--max-tokens` budget measures: each word-like run
/// counts one token, each JSON structural character (`{ } [ ] : , "`) half.
pub fn estimate_tokens(s: &str) -> usize {
    // Count word-like sequences (alphanumeric runs) as ~1 token each,
    // plus JSON structural characters ({, }, [, ], :, ,) as ~0.5 tokens each.
    let mut words = 0usize;
    let mut structural = 0usize;
    let mut in_word = false;

    for ch in s.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            if !in_word {
                words += 1;
                in_word = true;
            }
        } else {
            in_word = false;
            if matches!(ch, '{' | '}' | '[' | ']' | ':' | ',' | '"') {
                structural += 1;
            }
        }
    }

    // Each word ≈ 1 token, structural chars ≈ 0.5 tokens each (round up)
    words + structural.div_ceil(2)
}

#[cfg(test)]
mod tests;
