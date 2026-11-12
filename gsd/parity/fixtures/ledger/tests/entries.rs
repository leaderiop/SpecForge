use ledger::{add, line, parse, Entry};

#[test]
fn an_entry_keeps_its_cents_and_memo() {
    let mut entries = Vec::new();
    add(&mut entries, -1250, "coffee beans").unwrap();
    assert_eq!(
        entries,
        [Entry {
            cents: -1250,
            memo: "coffee beans".into()
        }]
    );
}

#[test]
fn a_memo_with_a_tab_is_refused() {
    assert!(add(&mut Vec::new(), 100, "a\tb").is_err());
}

#[test]
fn a_store_line_reads_back_as_its_entry() {
    let mut entries = Vec::new();
    add(&mut entries, 4200, "rent share").unwrap();
    assert_eq!(parse(&line(&entries[0])).unwrap(), entries);
}
