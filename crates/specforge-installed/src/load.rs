/// The builtin extensions the host embeds: name and component bytes.
#[derive(Debug, Clone, Copy)]
pub struct Builtins<'a>(pub &'a [(&'a str, &'a [u8])]);

impl<'a> Builtins<'a> {
    /// No builtin extension (a host that embeds none).
    pub const fn none() -> Self {
        Builtins(&[])
    }

    /// The embedded bytes of the builtin `name` (`@specforge/product`).
    pub fn get(&self, name: &str) -> Option<&'a [u8]> {
        self.0
            .iter()
            .find(|(builtin, _)| *builtin == name)
            .map(|(_, bytes)| *bytes)
    }

    /// Whether `name` is a builtin.
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
}
