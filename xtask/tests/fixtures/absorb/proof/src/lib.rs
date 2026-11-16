//! Three tests: two state two of an invariant's three obligations, one names it with no obligation's text.

#[cfg(test)]
mod tests {
    #[test]
    #[specforge_test_macros::test(
        invariant = "writes_are_durable",
        verify = "a killed writer leaves the old entry"
    )]
    fn a_killed_writer_leaves_the_old_entry() {}

    #[test]
    #[specforge_test_macros::test(invariant = "writes_are_durable", verify = "an acknowledged write is on disk")]
    fn an_acknowledged_write_is_on_disk() {}

    #[test]
    #[spec(invariant = "writes_are_durable")]
    fn names_the_invariant() {}
}
