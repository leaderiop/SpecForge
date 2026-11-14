//! The port's shrink-only ledgers, read one way (ADR 0332 D8, D9; ADR 0343 D1): a ledger is a text file of
//! tab-separated rows under `#` comment lines, and each row is a hit its test still allows. A hit with no row fails,
//! so nothing new comes in; a row with no hit fails, so the ledger only shrinks. No ledger has a bless.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

/// The rows of `text`: each line that is neither empty nor a `#` comment, cut into `cells` tab-separated cells, the
/// last taking the rest of the line. A row of fewer cells panics and names it.
pub fn rows(text: &str, cells: usize) -> Vec<Vec<String>> {
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let row: Vec<String> = l.splitn(cells, '\t').map(String::from).collect();
            assert_eq!(
                row.len(),
                cells,
                "a row of {cells} tab-separated cells: {l}"
            );
            row
        })
        .collect()
}

/// Fails unless `hits` and `rows` are the same multiset: it prints the hits with no row, in the ledger's own format
/// (ready to paste when a ledger is seeded), and the rows with no hit (to take off).
pub fn assert_rows(ledger: &str, hint: &str, hits: &[Vec<String>], rows: Vec<Vec<String>>) {
    let mut left = rows;
    let mut new = Vec::new();
    for hit in hits {
        match left.iter().position(|row| row == hit) {
            Some(k) => {
                left.swap_remove(k);
            }
            None => new.push(hit),
        }
    }
    let line = |row: &Vec<String>| row.join("\t");
    assert!(
        new.is_empty() && left.is_empty(),
        "{} hits not in {ledger} ({hint}):\n{}\n{} rows of {ledger} no longer hit (take them off):\n{}",
        new.len(),
        new.iter().map(|h| line(h)).collect::<Vec<_>>().join("\n"),
        left.len(),
        left.iter().map(line).collect::<Vec<_>>().join("\n")
    );
}
