//! Runtime-free testing of an extension's contributions. Lets authors pin
//! their handshake/describe wire output in plain unit tests without building
//! wasm or loading a host.

use crate::ContributionsBuilder;

/// Wraps a [`ContributionsBuilder`] and asserts on its wire output.
pub struct MockHost(pub ContributionsBuilder);

impl MockHost {
    pub fn new(builder: ContributionsBuilder) -> Self {
        Self(builder)
    }

    /// The handshake wire JSON this extension will report.
    pub fn handshake_json(&self) -> String {
        self.0.handshake_json()
    }

    /// The describe wire JSON for `category` (None for unsupported categories).
    pub fn describe_json(&self, category: &str) -> Option<String> {
        self.0.describe_response_json(category)
    }

    /// Assert the describe output for `category` equals `expected` as JSON
    /// (order-insensitive object comparison via serde_json::Value).
    pub fn assert_describe(&self, category: &str, expected: &str) {
        let actual = self.describe_json(category).unwrap_or_default();
        let a: serde_json::Value = serde_json::from_str(&actual).expect("actual is valid JSON");
        let e: serde_json::Value = serde_json::from_str(expected).expect("expected is valid JSON");
        assert_eq!(a, e, "describe '{category}' wire output mismatch");
    }

    /// Assert the handshake output equals `expected` as JSON.
    pub fn assert_handshake(&self, expected: &str) {
        let a: serde_json::Value =
            serde_json::from_str(&self.handshake_json()).expect("actual is valid JSON");
        let e: serde_json::Value = serde_json::from_str(expected).expect("expected is valid JSON");
        assert_eq!(a, e, "handshake wire output mismatch");
    }
}
