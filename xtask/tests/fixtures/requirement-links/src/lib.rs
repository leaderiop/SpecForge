//! A fixture crate whose tests name the requirements they prove, one attribute per requirement.

/// `a + b`.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// `a - b`.
pub fn sub(a: i32, b: i32) -> i32 {
    a - b
}

#[cfg(test)]
mod tests {
    use super::{add, sub};
    use specforge_test::prelude::*;

    #[specforge_test(requirement = "TRACER-01")]
    #[specforge_test(requirement = "TRACER-02")]
    fn adds() {
        assert_eq!(add(1, 2), 3);
    }

    #[specforge_test(requirement = "TRACER-02")]
    fn subtracts() {
        assert_eq!(sub(3, 2), 1);
    }
}
