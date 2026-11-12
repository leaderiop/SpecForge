//! A fixture crate the Rust packs collect.

#[path = "math/ops.rs"]
mod ops;

pub use ops::add;

#[cfg(test)]
mod tests {
    use super::add;
    use specforge_test::prelude::*;

    #[specforge_test(behavior = "rs_mark_tests")]
    fn adds() {
        assert_eq!(add(1, 2), 3);
    }

    #[test]
    #[ignore = "slow"]
    fn slow() {}
}
