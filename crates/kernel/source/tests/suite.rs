//! The source reader (ADR 0344): every test counts work and reads no clock.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

mod parse;
