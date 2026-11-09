fn main() {
    println!("{}", adder::add(1, 2));
}

#[cfg(test)]
mod tests {
    #[test]
    fn adds() {
        assert_eq!(adder::add(2, 2), 4);
    }
}
