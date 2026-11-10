use ledger::{column, total, Entry};

fn entry(cents: i64) -> Entry {
    Entry {
        cents,
        memo: "x".into(),
    }
}

#[test]
fn the_total_sums_every_entry() {
    assert_eq!(total(&[entry(1250), entry(-250), entry(99)]), 1099);
}

#[test]
fn total_beyond_i32_is_exact() {
    assert_eq!(
        total(&[entry(2_000_000_000), entry(2_000_000_000)]),
        4_000_000_000
    );
}

#[test]
fn a_total_prints_in_a_column_of_fourteen() {
    assert_eq!(column(-1099), "        -10.99");
}
