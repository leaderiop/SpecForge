//! The two readings of a bare word (ADR 0283): the integer it reads as and whether it is one identifier.

use super::Refusal;

pub(super) fn dashed_name() -> Refusal {
    Refusal::with("a name never starts with '-'")
}

/// The integer a bare word reads as: `-?N(-N)*`, each `N` digits, taken left to right as differences in checked
/// `i64` (`2-1` is 1, `03-02` is 1, `007` is 7, `2026-10-26` is 1990). `Err` names the operand that is none, or
/// the word when it leaves the `i64` range (ADR 0283).
pub fn integer_difference(word: &str) -> Result<i64, Refusal> {
    let (negative, rest) = match word.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, word),
    };
    let outside = || Refusal::with(&format!("'{word}' is outside {}..{}", i64::MIN, i64::MAX));
    let mut total: Option<i64> = None;
    for (i, term) in rest.split('-').enumerate() {
        let n = if !term.is_empty() && term.bytes().all(|b| b.is_ascii_digit()) {
            let n: i64 = term.parse().map_err(|_| outside())?;
            if i == 0 && negative { -n } else { n }
        } else if term.contains('.')
            && term
                .split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(Refusal::with(
                "an integer has no fraction (there is no decimal type)",
            ));
        } else {
            return Err(Refusal::with(&format!("'{term}' is no number")));
        };
        total = Some(match total {
            None => n,
            Some(t) => t.checked_sub(n).ok_or_else(outside)?,
        });
    }
    total.ok_or_else(|| Refusal::with(&format!("'{word}' is no number")))
}

/// Whether `text` is one word: the grammar's identifier (letters, digits and
/// `_`, starting with a letter or `_`).
pub fn is_word(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
