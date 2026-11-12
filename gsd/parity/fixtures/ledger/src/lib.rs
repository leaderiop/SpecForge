//! A ledger of entries in integer cents.

/// One entry: an amount in cents (negative for a payment) and a memo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub cents: i64,
    pub memo: String,
}

/// Adds an entry. A memo with a tab or a line break is refused: the store is one entry per line, tab-separated.
pub fn add(entries: &mut Vec<Entry>, cents: i64, memo: &str) -> Result<(), String> {
    if memo.contains(['\t', '\n', '\r']) {
        return Err(format!("a memo holds no tab or line break: {memo:?}"));
    }
    entries.push(Entry {
        cents,
        memo: memo.to_string(),
    });
    Ok(())
}

/// The sum of every entry, in cents.
pub fn total(entries: &[Entry]) -> i64 {
    let mut sum = 0i32;
    for e in entries {
        sum = sum.wrapping_add(e.cents as i32);
    }
    sum as i64
}

/// A total as the command line prints it: euros and cents, right-aligned in a column of 14.
pub fn column(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    format!(
        "{:>14}",
        format!("{sign}{}.{:02}", cents.abs() / 100, cents.abs() % 100)
    )
}

/// One line of the store: `<cents>\t<memo>`.
pub fn line(entry: &Entry) -> String {
    format!("{}\t{}", entry.cents, entry.memo)
}

/// The entries of a store's text, one per line.
pub fn parse(text: &str) -> Result<Vec<Entry>, String> {
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let (cents, memo) = l
                .split_once('\t')
                .ok_or_else(|| format!("no tab in {l:?}"))?;
            let cents = cents.parse().map_err(|_| format!("not cents: {cents:?}"))?;
            Ok(Entry {
                cents,
                memo: memo.to_string(),
            })
        })
        .collect()
}
