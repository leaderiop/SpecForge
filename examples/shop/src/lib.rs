//! The demo cart. Its tests name the spec obligations they prove with
//! `#[specforge_test]`; `specforge collect` records the results.

/// Add `name` to the cart.
pub fn add(cart: &mut Vec<String>, name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("empty name");
    }
    if cart.iter().any(|item| item == name) {
        return Err("already in the cart");
    }
    cart.push(name.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test::prelude::*;

    #[specforge_test(type = "cart", verify = "starts empty")]
    fn starts_empty() {
        assert!(Vec::<String>::new().is_empty());
    }

    #[specforge_test(behavior = "add_item", verify = "adds the item")]
    fn adds_the_item() {
        let mut cart = Vec::new();
        add(&mut cart, "apple").unwrap();
        assert_eq!(cart, ["apple"]);
    }

    #[specforge_test(behavior = "add_item", verify = "rejects an empty name")]
    fn rejects_an_empty_name() {
        assert!(add(&mut Vec::new(), "").is_err());
    }

    // "rejects a duplicate item" has no test yet: `specforge analyze`
    // reports it (A015). The invariant's property is tested here.
    #[specforge_test(
        invariant = "no_duplicate_items",
        verify = "a second add of the same item fails"
    )]
    fn a_second_add_fails() {
        let mut cart = Vec::new();
        add(&mut cart, "apple").unwrap();
        assert!(add(&mut cart, "apple").is_err());
    }
}
